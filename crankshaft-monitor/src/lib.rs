//! Crate for monitoring crankshaft events.
use std::net::SocketAddr;

use anyhow;
use proto::{Event, monitor_server::MonitorServer};
use server::MonitorService;
use tokio::sync::broadcast;

pub mod proto;
pub mod server;

const DEFAULT_CHANNEL_CAPACITY: usize = 16;

/// The main external API to start the Crankshaft monitor.
pub fn start_monitoring(
    addr: SocketAddr,
) -> Result<
    (
        broadcast::Sender<Event>,
        tokio::task::JoinHandle<Result<(), tonic::transport::Error>>,
    ),
    anyhow::Error,
> {
    let (event_sender, event_receiver) = broadcast::channel(DEFAULT_CHANNEL_CAPACITY);
    let monitor_service = MonitorService::new(event_receiver);
    let server = MonitorServer::new(monitor_service);

    let server_handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(server)
            .serve(addr)
            .await
    });

    Ok((event_sender, server_handle))
}
