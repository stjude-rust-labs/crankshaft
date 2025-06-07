//! Crate for monitoring crankshaft events.
use std::net::SocketAddr;

use proto::{Event, monitor_service_server::MonitorServiceServer};
use server::CrankshaftMonitorServer;
use tokio::sync::broadcast;

/// protobuf module.
pub mod proto;
/// server module.
pub mod server;

/// The main external Api to start the crankshaft monitor.
pub async fn start_monitoring(
    addr: SocketAddr,
) -> Result<
    (
        broadcast::Sender<Event>,
        tokio::task::JoinHandle<Result<(), tonic::transport::Error>>,
    ),
    Box<dyn std::error::Error>,
> {
    let (event_sender, event_receiver) = broadcast::channel(16);
    let monitor_service = CrankshaftMonitorServer::new(event_receiver);
    let server = MonitorServiceServer::new(monitor_service);

    let server_handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(server)
            .serve(addr)
            .await
    });

    Ok((event_sender, server_handle))
}
