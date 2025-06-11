//! The module for the actual gRPC server
use std::pin::Pin;

use crate::proto::monitor_server::Monitor;
use crate::proto::{Event, SubscribeEventsRequest};
use futures_core::Stream;
use tokio::sync::broadcast;
use tonic::{Request, Response, Status};

/// The MonitorService struct represents a gRPC service for monitoring events.
pub struct MonitorService {
    /// The receiver field is a broadcast::Receiver<Event> that receives events to stream to clients.
    pub receiver: broadcast::Receiver<Event>,
}

impl MonitorService {
    /// Creates a new instance of CrankshaftMonitorServer with the given receiver.
    pub fn new(receiver: broadcast::Receiver<Event>) -> Self {
        Self { receiver }
    }
}

#[tonic::async_trait]
impl Monitor for MonitorService {
    type SubscribeEventsStream = Pin<Box<dyn Stream<Item = Result<Event, Status>> + Send>>;

    /// Subscribes to all task events, streaming them to clients (e.g., TUI or web).
    /// Clients can use a for loop like:
    /// let response = client.subscribe_events(request).await.expect("Stream failed");
    /// let mut stream = response.into_inner();
    /// while let Some(event) = stream.next().await { ... }
    async fn subscribe_events(
        &self,
        _request: Request<SubscribeEventsRequest>,
    ) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        let mut receiver = self.receiver.resubscribe();

        let stream = async_stream::stream! {
            while let Ok(event) = receiver.recv().await {
                yield Ok(event);
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }
}
