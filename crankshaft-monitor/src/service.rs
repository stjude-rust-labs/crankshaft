//! Implements the gRPC monitor service.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::SystemTime;

use crankshaft_events::Event as CrankshaftEvent;
use futures_core::Stream;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tokio_util::sync::CancellationToken;
use tonic::Code;
use tonic::Request;
use tonic::Response;
use tonic::Result;
use tonic::Status;
use tracing::error;

use crate::proto::CancelTaskRequest;
use crate::proto::CancelTaskResponse;
use crate::proto::Event;
use crate::proto::ExitStatus;
use crate::proto::ServiceStateRequest;
use crate::proto::ServiceStateResponse;
use crate::proto::SubscribeEventsRequest;
use crate::proto::TaskCanceledEvent;
use crate::proto::TaskCompletedEvent;
use crate::proto::TaskContainerCreatedEvent;
use crate::proto::TaskContainerExitedEvent;
use crate::proto::TaskCreatedEvent;
use crate::proto::TaskEvents;
use crate::proto::TaskFailedEvent;
use crate::proto::TaskPreemptedEvent;
use crate::proto::TaskStartedEvent;
use crate::proto::TaskStderrEvent;
use crate::proto::TaskStdoutEvent;
use crate::proto::event::EventKind;
use crate::proto::exit_status::ExitStatusKind;
use crate::proto::monitor_server::Monitor;

/// Represents a task identifier.
pub type TaskId = u64;

/// Helper trait for converting Crankshaft types into Protobuf types.
trait IntoProtobuf<T> {
    /// Converts the type into its Protobuf representation.
    fn into_protobuf(self) -> T;
}

impl IntoProtobuf<ExitStatusKind> for std::process::ExitStatus {
    fn into_protobuf(self) -> ExitStatusKind {
        if let Some(code) = self.code() {
            ExitStatusKind::Code(code)
        } else {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                ExitStatusKind::Signal(self.signal().expect("exit should be from a signal"))
            }
            #[cfg(not(unix))]
            {
                panic!("failed to retrieve exit code");
            }
        }
    }
}

impl IntoProtobuf<ExitStatus> for std::process::ExitStatus {
    fn into_protobuf(self) -> ExitStatus {
        ExitStatus {
            exit_status_kind: Some(self.into_protobuf()),
        }
    }
}

impl IntoProtobuf<EventKind> for CrankshaftEvent {
    fn into_protobuf(self) -> EventKind {
        match self {
            CrankshaftEvent::TaskCreated {
                id, name, tes_id, ..
            } => EventKind::Created(TaskCreatedEvent { id, name, tes_id }),
            CrankshaftEvent::TaskStarted { id } => EventKind::Started(TaskStartedEvent { id }),
            CrankshaftEvent::TaskContainerCreated { id, container } => {
                EventKind::ContainerCreated(TaskContainerCreatedEvent { id, container })
            }
            CrankshaftEvent::TaskContainerExited {
                id,
                container,
                exit_status,
            } => EventKind::ContainerExited(TaskContainerExitedEvent {
                id,
                container,
                exit_status: Some(exit_status.into_protobuf()),
            }),
            CrankshaftEvent::TaskCompleted { id, exit_statuses } => {
                EventKind::Completed(TaskCompletedEvent {
                    id,
                    exit_statuses: exit_statuses
                        .into_iter()
                        .map(IntoProtobuf::into_protobuf)
                        .collect(),
                })
            }
            CrankshaftEvent::TaskFailed { id, message } => {
                EventKind::Failed(TaskFailedEvent { id, message })
            }
            CrankshaftEvent::TaskCanceled { id } => EventKind::Canceled(TaskCanceledEvent { id }),
            CrankshaftEvent::TaskPreempted { id } => {
                EventKind::Preempted(TaskPreemptedEvent { id })
            }
            CrankshaftEvent::TaskStdout { id, message } => EventKind::Stdout(TaskStdoutEvent {
                id,
                message: message.to_vec(),
            }),
            CrankshaftEvent::TaskStderr { id, message } => EventKind::Stderr(TaskStderrEvent {
                id,
                message: message.to_vec(),
            }),
        }
    }
}

impl IntoProtobuf<Event> for CrankshaftEvent {
    fn into_protobuf(self) -> Event {
        Event {
            timestamp: Some(SystemTime::now().into()),
            event_kind: Some(self.into_protobuf()),
        }
    }
}

/// The state maintained by the monitor service.
#[derive(Debug, Default)]
pub struct ServiceState {
    /// The map of task identifier to its events.
    tasks: HashMap<TaskId, TaskEvents>,
    /// The map of task identifier to its CancellationToken
    tokens: HashMap<TaskId, CancellationToken>,
}

/// Represents a gRPC service for monitoring Crankshaft events in real-time.
#[derive(Debug)]
pub struct MonitorService {
    /// The events sender from Crankshaft.
    ///
    /// This is used to subscribe new receivers for clients.
    tx: broadcast::Sender<CrankshaftEvent>,
    /// The current state of the service.
    state: Arc<RwLock<ServiceState>>,
    /// The cancellation token for shutting down the service.
    token: CancellationToken,
}

impl MonitorService {
    /// Creates a new monitor service.
    pub fn new(
        tx: broadcast::Sender<CrankshaftEvent>,
        rx: broadcast::Receiver<CrankshaftEvent>,
        token: CancellationToken,
    ) -> Self {
        let state: Arc<RwLock<ServiceState>> = Arc::default();
        tokio::spawn(Self::update_state(rx, state.clone(), token.clone()));
        Self { tx, state, token }
    }

    /// Handles service state updates.
    async fn update_state(
        mut events: broadcast::Receiver<CrankshaftEvent>,
        state: Arc<RwLock<ServiceState>>,
        token: CancellationToken,
    ) {
        loop {
            tokio::select! {
                _ = token.cancelled() => break,
                r = events.recv() => match r {
                    Ok(event) => {
                        let (id, remove) = match event {
                            CrankshaftEvent::TaskCreated { id, ref token, .. } => {
                                state.write().await.tokens.insert(id, token.clone());
                                (id, false)
                            },
                            CrankshaftEvent::TaskStarted { id }
                            | CrankshaftEvent::TaskContainerCreated { id, .. }
                            | CrankshaftEvent::TaskContainerExited { id, .. }
                            | CrankshaftEvent::TaskStdout { id, .. }
                            | CrankshaftEvent::TaskStderr { id, .. } => (id, false),
                            CrankshaftEvent::TaskCompleted { id, .. }
                            | CrankshaftEvent::TaskFailed { id, .. }
                            | CrankshaftEvent::TaskCanceled { id }
                            | CrankshaftEvent::TaskPreempted { id } => (id, true),
                        };


                        if remove {
                            let mut state = state.write().await;
                            state.tasks.remove(&id);
                        } else {
                            let event: Event = event.into_protobuf();
                            let mut state = state.write().await;
                            let task = state.tasks.entry(id).or_default();

                            // If there aren't any events, ensure the first one is the created event
                            if task.events.is_empty() {
                                if let Some(EventKind::Created(_)) = &event.event_kind {
                                    task.events.push(event);
                                }
                            } else {
                                task.events.push(event);
                            }
                        }
                    }
                    Err(RecvError::Closed) => break,
                    Err(e) => {
                        error!("failed to read from event stream: {e:#}");
                        continue;
                    }
                }
            }
        }
    }
}

#[tonic::async_trait]
impl Monitor for MonitorService {
    type SubscribeEventsStream = Pin<Box<dyn Stream<Item = Result<Event, Status>> + Send>>;

    async fn subscribe_events(
        &self,
        _request: Request<SubscribeEventsRequest>,
    ) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        let mut receiver = self.tx.subscribe();
        let token = self.token.clone();

        let stream = async_stream::stream! {
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    r = receiver.recv() => match r {
                        Ok(event) => yield Ok(event.into_protobuf()),
                        Err(RecvError::Closed) => break,
                        Err(e) => yield Err(Status::new(Code::Internal, e.to_string()))
                    }
                }
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }

    async fn get_service_state(
        &self,
        _: Request<ServiceStateRequest>,
    ) -> Result<Response<ServiceStateResponse>, Status> {
        let state = self.state.read().await;
        Ok(Response::new(ServiceStateResponse {
            tasks: state.tasks.clone(),
        }))
    }

    async fn cancel_task(
        &self,
        request: Request<CancelTaskRequest>,
    ) -> Result<Response<CancelTaskResponse>, Status> {
        let id = request.into_inner().id;

        let mut state = self.state.write().await;

        if let Some(token) = state.tokens.remove(&id) {
            token.cancel();
            state.tasks.remove(&id);

            Ok(Response::new(CancelTaskResponse { success: true }))
        } else {
            Err(Status::not_found(format!("Task {id} not found")))
        }
    }
}
