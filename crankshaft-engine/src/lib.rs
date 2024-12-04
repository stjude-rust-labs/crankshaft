//! The engine that powers Crankshaft.

use std::time::Duration;

use crankshaft_config::backend::Config;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use indexmap::IndexMap;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
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
/// different error type may be returned (as it will always be coercable to it's
/// [`anyhow`] equivalent for display).
pub type Result<T> = eyre::Result<T>;

/// Runners stored within the engine.
type Runners = IndexMap<String, Runner>;

/// A workflow execution engine.
#[derive(Debug, Default)]
pub struct Engine {
    /// The task runner(s).
    runners: Runners,
}

impl Engine {
    /// Adds a [`Backend`] to the engine.
    pub async fn with(mut self, config: Config) -> Result<Self> {
        let (name, kind, max_tasks, defaults) = config.into_parts();
        let runner = Runner::initialize(kind, max_tasks, defaults).await?;
        self.runners.insert(name, runner);
        Ok(self)
    }

    /// Gets the names of the runners.
    pub fn runners(&self) -> impl Iterator<Item = &str> {
        self.runners.keys().map(|key| key.as_ref())
    }

    /// Submits a [`Task`] to be executed.
    ///
    /// A [`Handle`] is returned, which contains a channel that can be awaited
    /// for the result of the job.
    pub fn submit(&self, name: impl AsRef<str>, task: Task) -> TaskHandle {
        let name = name.as_ref();
        let backend = self
            .runners
            .get(name)
            .unwrap_or_else(|| panic!("backend not found: {name}"));

        debug!(
            "submitting job{} to the `{}` backend",
            task.name()
                .map(|name| format!(" with name `{}`", name))
                .unwrap_or_default(),
            name
        );

        backend.submit(task)
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

    /// Runs all of the tasks scheduled in the engine.
    pub async fn run(self) {
        let mut futures = FuturesUnordered::new();

        for (_, runner) in self.runners {
            futures.extend(runner.tasks());
        }

        let task_completion_bar = ProgressBar::new(futures.len() as u64);
        task_completion_bar.set_style(
            ProgressStyle::with_template(
                "{spinner:.cyan/blue} [{elapsed_precise}] [{wide_bar:.cyan/blue}] \
                 {pos:>7}/{len:7} {msg}",
            )
            .unwrap()
            .progress_chars("#>-"),
        );

        let mut count = 1;
        task_completion_bar.inc(0);
        task_completion_bar.enable_steady_tick(Duration::from_millis(100));

        while (futures.next().await).is_some() {
            task_completion_bar.set_message(format!("task #{}", count));
            task_completion_bar.inc(1);
            count += 1;
        }
    }
}
