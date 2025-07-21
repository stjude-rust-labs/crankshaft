//! The module for the actual gRPC server
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use futures_core::Stream;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tonic::Request;
use tonic::Response;
use tonic::Result;
use tonic::Status;

use crate::proto::Event;
use crate::proto::EventType;
use crate::proto::GetServerStateRequest;
use crate::proto::Resources;
use crate::proto::ServerStateResponse;
use crate::proto::SubscribeEventsRequest;
use crate::proto::TaskState;
use crate::proto::event::Payload;
use crate::proto::monitor_server::Monitor;

/// The resource struct
#[derive(Clone, Debug, Default)]
pub struct Resource {
    /// number of nodes
    pub nodes: f64,
    /// cpu
    pub cpu: f64,
    /// memory
    pub memory: f64,
    /// max_cpu
    pub max_cpu: f64,
    /// max_memory
    pub max_memory: f64,
}

/// The TaskId
pub type Taskid = String;

/// The ServerState struct represents the Resource and Tasks state of the server
#[derive(Default)]
pub struct ServerState {
    /// resources info  about the server
    resources: Resource,

    /// tasks is a hashmap for taskId -> TaskState
    tasks: HashMap<Taskid, i32>,
}

/// The MonitorService struct represents a gRPC service for monitoring events.
pub struct MonitorService {
    /// The receiver field is a `broadcast::Receiver<Event>` that receives
    /// events to stream to clients.
    pub receiver: broadcast::Receiver<Event>,
    /// Current state of the server
    pub state: Arc<RwLock<ServerState>>,
}

impl MonitorService {
    /// Creates a new instance of CrankshaftMonitorServer with the given
    /// receiver.
    pub fn new(receiver: broadcast::Receiver<Event>, state: Arc<RwLock<ServerState>>) -> Self {
        Self { receiver, state }
    }
}

#[tonic::async_trait]
impl Monitor for MonitorService {
    type SubscribeEventsStream = Pin<Box<dyn Stream<Item = Result<Event, Status>> + Send>>;

    /// Subscribes to all task events, streaming them to clients (e.g., TUI or
    /// web). Clients can use a for loop like:
    /// let response = client.subscribe_events(request).await.expect("Stream
    /// failed"); let mut stream = response.into_inner();
    /// while let Some(event) = stream.next().await { ... }
    async fn subscribe_events(
        &self,
        _request: Request<SubscribeEventsRequest>,
    ) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        let mut receiver = self.receiver.resubscribe();
        let state = self.state.clone(); // clone the Arc

        let stream = async_stream::stream! {
            while let Ok(event) = receiver.recv().await {
                match event.event_type() {
                    EventType::ContainerStarted | EventType::ServiceStarted => {
                        if let Some(Payload::Resources(resource)) = event.payload.clone() {
                            let mut state = state.write().await;
                            state.resources = crate::server::Resource {
                                nodes: resource.nodes,
                                cpu: resource.cpu,
                                memory: resource.memory,
                                max_cpu: resource.max_cpu,
                                max_memory: resource.max_memory,
                            };
                        }
                    },
                    EventType::TaskQueued => {
                        let mut state = state.write().await;
                        state.tasks.insert(event.event_id.clone(), TaskState::Queued as i32);
                    }
                    EventType::TaskStarted => {
                        let mut state = state.write().await;
                        state.tasks.insert(event.event_id.clone(), TaskState::Started as i32);
                    }
                    EventType::TaskCompleted => {
                        let mut state = state.write().await;
                        state.tasks.insert(event.event_id.clone(), TaskState::Completed as i32);
                    }
                    EventType::TaskFailed => {
                        let mut state = state.write().await;
                        state.tasks.insert(event.event_id.clone(), TaskState::Failed as i32);
                    }
                    EventType::TaskStopped => {
                        let mut state = state.write().await;
                        state.tasks.insert(event.event_id.clone(), TaskState::Stopped as i32);
                    }
                    _ => (),
                }

                yield Ok(event);
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }

    async fn get_server_state(
        &self,
        _request: Request<GetServerStateRequest>,
    ) -> Result<Response<ServerStateResponse>, Status> {
        let state = self.state.read().await;

        Ok(Response::new(ServerStateResponse {
            resources: Some(Resources {
                nodes: state.resources.nodes,
                cpu: state.resources.cpu,
                memory: state.resources.memory,
                max_cpu: state.resources.max_cpu,
                max_memory: state.resources.max_memory,
            }),
            tasks: state.tasks.clone(),
        }))
    }
}
