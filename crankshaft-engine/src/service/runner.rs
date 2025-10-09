//! Task runner services.

use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use crankshaft_config::backend::Defaults;
use crankshaft_config::backend::Kind;
use crankshaft_events::Event;
use nonempty::NonEmpty;
use tokio::sync::Semaphore;
use tokio::sync::broadcast;
use tokio::sync::oneshot::Receiver;
use tokio_util::sync::CancellationToken;
use tracing::trace;

pub mod backend;

pub use backend::Backend;

use crate::Task;
use crate::service::name::GeneratorIterator;
use crate::service::name::UniqueAlphanumeric;
use crate::service::runner::backend::docker;
use crate::service::runner::backend::generic;
use crate::service::runner::backend::tes;

/// The size of the name buffer.
const NAME_BUFFER_LEN: usize = 4096;

/// A spawned task handle.
#[derive(Debug)]
pub struct TaskHandle(Receiver<Result<NonEmpty<ExitStatus>, backend::TaskRunError>>);

impl TaskHandle {
    /// Consumes the task handle and waits for the task to complete.
    ///
    /// Returns the exit statuses of the task's executors.
    pub async fn wait(self) -> Result<NonEmpty<ExitStatus>, backend::TaskRunError> {
        self.0
            .await
            .map_err(|e| backend::TaskRunError::Other(e.into()))?
    }
}

/// A generic task runner.
#[derive(Debug)]
pub struct Runner {
    /// The task runner itself.
    backend: Arc<dyn Backend>,
    /// The task lock.
    lock: Arc<tokio::sync::Semaphore>,
}

impl Runner {
    /// Creates a new [`Runner`].
    pub async fn initialize(
        config: Kind,
        max_tasks: usize,
        defaults: Option<Defaults>,
        events: Option<broadcast::Sender<Event>>,
    ) -> Result<Self> {
        let names = Arc::new(Mutex::new(GeneratorIterator::new(
            UniqueAlphanumeric::default_with_expected_generations(NAME_BUFFER_LEN),
            NAME_BUFFER_LEN,
        )));

        let backend = match config {
            Kind::Docker(config) => {
                let backend =
                    docker::Backend::initialize_default_with(config, names, events).await?;
                Arc::new(backend) as Arc<dyn Backend>
            }
            Kind::Generic(config) => {
                let backend = generic::Backend::initialize(config, defaults, names, events).await?;
                Arc::new(backend)
            }
            Kind::TES(config) => Arc::new(tes::Backend::initialize(config, names, events).await),
        };

        Ok(Self {
            backend,
            lock: Arc::new(Semaphore::new(max_tasks)),
        })
    }

    /// Spawns a task to be executed by the backend.
    ///
    /// The `started` callback is called for each execution of the task that has
    /// started; the parameter is the index of the execution from the task's
    /// executions collection.
    ///
    /// The `cancellation` token can be used to gracefully cancel the task.
    pub async fn spawn(&self, task: Task, token: CancellationToken) -> anyhow::Result<TaskHandle> {
        trace!(backend = ?self.backend, task = ?task);

        let (tx, rx) = tokio::sync::oneshot::channel();
        let backend = self.backend.clone();
        let lock = self.lock.clone();

        tokio::spawn(async move {
            let _permit = lock.acquire().await?;
            let result = backend.clone().run(task, token)?.await;

            // NOTE: if the send does not succeed, that is almost certainly
            // because the receiver was dropped. That is a relatively standard
            // practice if you don't specifically _want_ to keep a handle to the
            // returned result, so we ignore any errors related to that.
            let _ = tx.send(result);
            drop(_permit);
            anyhow::Ok(())
        });

        Ok(TaskHandle(rx))
    }
}
