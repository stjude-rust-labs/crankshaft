//! The engine that powers Crankshaft.

use anyhow::Context;
use anyhow::Result;
use crankshaft_config::backend::Config;
use crankshaft_events::Event;
use indexmap::IndexMap;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::debug;

pub mod service;
pub mod task;

pub use task::Task;

use crate::service::Runner;
use crate::service::runner::Backend;
use crate::service::runner::TaskHandle;

/// The capacity for the events channel.
///
/// This is the number of events to buffer in the channel before receivers
/// become lagged.
///
/// The value of `100` was chosen simply as a reasonable default.
const EVENTS_CHANNEL_CAPACITY: usize = 100;

/// A workflow execution engine.
#[derive(Debug)]
pub struct Engine {
    /// The task runner(s).
    runners: IndexMap<String, Runner>,
    /// The events sender.
    events: Option<broadcast::Sender<Event>>,
    /// The monitor for the engine.
    #[cfg(feature = "monitoring")]
    monitor: Option<crankshaft_monitor::Monitor>,
}

impl Engine {
    /// Constructs a new engine.
    pub fn new() -> Self {
        let (events_tx, _) = broadcast::channel(EVENTS_CHANNEL_CAPACITY);
        Self {
            runners: Default::default(),
            events: Some(events_tx),
            #[cfg(feature = "monitoring")]
            monitor: None,
        }
    }

    /// Constructs a new engine with monitoring enabled.
    #[cfg(feature = "monitoring")]
    pub fn new_with_monitoring(addr: std::net::SocketAddr) -> Self {
        let (events_tx, _) = broadcast::channel(EVENTS_CHANNEL_CAPACITY);
        let monitor = crankshaft_monitor::Monitor::start(addr, events_tx.clone());

        Self {
            runners: Default::default(),
            events: Some(events_tx),
            monitor: Some(monitor),
        }
    }

    /// Adds a [`Backend`] to the engine.
    pub async fn with(mut self, config: Config) -> Result<Self> {
        let (name, kind, max_tasks, defaults) = config.into_parts();
        let runner = Runner::initialize(kind, max_tasks, defaults, self.events.clone()).await?;
        self.runners.insert(name, runner);
        Ok(self)
    }

    /// Subscribes to the engine's events and returns a receiver.
    ///
    /// Returns an error if the engine has already been shut down.
    pub fn subscribe(&self) -> Result<broadcast::Receiver<Event>> {
        Ok(self
            .events
            .as_ref()
            .context("engine has shut down")?
            .subscribe())
    }

    /// Gets the names of the runners.
    pub fn runners(&self) -> impl Iterator<Item = &str> {
        self.runners.keys().map(|key| key.as_ref())
    }

    /// Shuts down the engine.
    pub async fn shutdown(mut self) {
        // Drop the events sender
        self.events.take();

        #[cfg(feature = "monitoring")]
        if let Some(monitor) = self.monitor.take() {
            monitor.stop().await;
        }
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
            "submitting job{job} to the `{name}` backend",
            job = task
                .name
                .as_ref()
                .map(|name| format!(" with name `{name}`"))
                .unwrap_or_default(),
        );

        backend.spawn(task, token)
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

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Drop the events sender before the monitor
        self.events.take();
    }
}
