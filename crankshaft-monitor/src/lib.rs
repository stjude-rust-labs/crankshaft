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
    /// The cancellation token for shutting down the server.
    token: CancellationToken,
}

impl Monitor {
    /// Starts the monitor and binds it to the given address.
    ///
    /// The provided events stream will be subscribed to when clients connect to
    /// the monitor.
    ///
    /// Note that a failure to bind to the address will disable monitoring.
    pub async fn start(addr: SocketAddr, events: broadcast::Sender<Event>) -> Self {
        // Immediately subscribe here before we spawn the server task; this allows the
        // server to receive all events after `start` is called
        let rx = events.subscribe();

        let token = CancellationToken::new();
        let server = tokio::spawn(Self::run_server(addr, events, rx, token.clone()));

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
        tx: broadcast::Sender<Event>,
        rx: broadcast::Receiver<Event>,
        token: CancellationToken,
    ) {
        let service = MonitorService::new(tx, rx, token.clone()).await;
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
