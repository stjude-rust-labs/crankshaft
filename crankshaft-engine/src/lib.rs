//! The engine that powers Crankshaft.

use std::net::SocketAddr;

use anyhow::Result;
use crankshaft_config::backend::Config;
use crankshaft_monitor::proto::Event;
use crankshaft_monitor::start_monitoring;
use indexmap::IndexMap;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::debug;

pub mod service;
pub mod task;

pub use task::Task;

use crate::service::Runner;
use crate::service::runner::Backend;
use crate::service::runner::TaskHandle;

/// A workflow execution engine.
#[derive(Debug, Default)]
pub struct Engine {
    /// The task runner(s).
    runners: IndexMap<String, Runner>,
    /// The monitoring server sender, if monitoring is enabled.
    monitoring_sender: Option<broadcast::Sender<Event>>,
    /// The monitoring server task handle, if monitoring is enabled.
    monitoring_handle: Option<JoinHandle<Result<(), tonic::transport::Error>>>,
}

impl Engine {
    /// Adds a [`Backend`] to the engine.
    pub async fn with(mut self, config: Config) -> Result<Self> {
        let (name, kind, max_tasks, defaults, monitored) = config.into_parts();
        let runner = Runner::initialize(kind, max_tasks, defaults, monitored).await?;
        self.runners.insert(name, runner);

        // Start the monitoring server if any runner is monitored
        if monitored && self.monitoring_sender.is_none() {
            let socketaddr: SocketAddr = "127.0.0.1:8080".parse()?;
            let (event_sender, handle) = start_monitoring(socketaddr)?;
            self.monitoring_sender = Some(event_sender);
            self.monitoring_handle = Some(handle);
        }

        Ok(self)
    }

    /// Gets the names of the runners.
    pub fn runners(&self) -> impl Iterator<Item = &str> {
        self.runners.keys().map(|key| key.as_ref())
    }

    /// Spawns a [`Task`] to be executed.
    ///
    /// The `cancellation` token can be used to gracefully cancel the task.
    ///
    /// A [`TaskHandle`] is returned, which contains a channel that can be
    /// awaited for the result of the job.
    pub fn spawn(
        &self,
        name: impl AsRef<str>,
        task: Task,
        token: CancellationToken,
    ) -> Result<TaskHandle> {
        let name = name.as_ref();
        let backend = self
            .runners
            .get(name)
            .unwrap_or_else(|| panic!("backend not found: {name}"));

        debug!(
            "submitting job{} to the `{}` backend",
            task.name
                .as_ref()
                .map(|name| format!(" with name `{name}`"))
                .unwrap_or_default(),
            name
        );

        let event_sender = if backend.monitored {
            self.monitoring_sender.clone()
        } else {
            None
        };

        backend.spawn(task, event_sender, token)
    }

    /// Starts an instrumentation loop.
    #[cfg(tokio_unstable)]
    pub fn start_instrument(delay_ms: u64) {
        use tokio_metrics::RuntimeMonitor;
        use tracing::info;

        let handle = tokio::runtime::Handle::current();
        let monitor = RuntimeMonitor::new(&handle);

        tokio::spawn(async move {
            for interval in monitor.intervals() {
                info!("{:?}", interval.total_park_count);
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        });
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Some(handle) = self.monitoring_handle.take() {
            debug!("Shutting down monitoring server");
            handle.abort();
        }
    }
}
