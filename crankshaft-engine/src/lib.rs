//! The engine that powers Crankshaft.

use crankshaft_config::backend::Config;
use indexmap::IndexMap;
use tokio_util::sync::CancellationToken;
use tracing::debug;

pub mod service;
pub mod task;

pub use task::Task;

use crate::service::Runner;
use crate::service::runner::Backend;
use crate::service::runner::TaskHandle;

/// The top-level result returned within the engine.
///
/// An [`eyre::Result`] was chosen as the top-level result for the engine
/// simply because engine errors are not typically recoverable—they will usually
/// be displayed directly to the user.
///
/// In cases where an error may be recoverable throughout this crate, a
/// different error type may be returned (as it will always be coercible to it's
/// [`anyhow`] equivalent for display).
pub type Result<T> = eyre::Result<T>;

/// A workflow execution engine.
#[derive(Debug, Default)]
pub struct Engine {
    /// The task runner(s).
    runners: IndexMap<String, Runner>,
}

impl Engine {
    /// Adds a [`Backend`] to the engine.
    pub async fn with(mut self, config: Config) -> Result<Self> {
        let runner = Runner::initialize(config.kind, config.max_tasks, config.defaults).await?;
        self.runners.insert(config.name, runner);
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
    /// A [`Handle`] is returned, which contains a channel that can be awaited
    /// for the result of the job.
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
                .map(|name| format!(" with name `{}`", name))
                .unwrap_or_default(),
            name
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
