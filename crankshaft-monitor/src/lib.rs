//! Crate for monitoring crankshaft events.
use std::net::SocketAddr;

use crankshaft_events::Event;
use proto::monitor_server::MonitorServer;
use service::MonitorService;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::warn;

pub mod proto;
mod service;

/// Represents a monitor of Crankshaft events.
///
/// The monitor exposes a gRPC service for subscribing to Crankshaft events.
#[derive(Debug)]
pub struct Monitor {
    /// The handle to the task running the gRPC server.
    server: Option<JoinHandle<()>>,
    /// The cancellation token for shutting down the task.
    token: CancellationToken,
}

impl Monitor {
    /// Starts the monitor and binds it to the given address.
    ///
    /// Note that a failure to bind to the address will disable monitoring.
    pub fn start(addr: SocketAddr, events: broadcast::Receiver<Event>) -> Self {
        let token = CancellationToken::new();
        let server = tokio::spawn(Self::run_server(addr, events, token.clone()));

        Self {
            server: Some(server),
            token,
        }
    }

    /// Stops the monitor.
    pub async fn stop(mut self) {
        // Cancel the task
        self.token.cancel();

        // Wait for the server task to join
        self.server
            .take()
            .unwrap()
            .await
            .expect("server task panicked");
    }

    /// Runs the gRPC server.
    async fn run_server(
        addr: SocketAddr,
        events: broadcast::Receiver<Event>,
        token: CancellationToken,
    ) {
        let service = MonitorService::new(events);
        let server = MonitorServer::new(service);

        info!("starting Crankshaft monitor at http://{addr}");

        if let Err(e) = tonic::transport::Server::builder()
            .add_service(server)
            .serve_with_shutdown(addr, token.cancelled())
            .await
        {
            warn!("failed to bind monitoring service: {e} (monitoring is disabled)");
            return;
        }

        info!("Crankshaft monitor has shut down");
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.token.cancel();
    }
}
