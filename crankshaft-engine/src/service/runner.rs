//! Task runner services.

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
use crate::task::ExecutionResult;

/// The size of the name buffer.
const NAME_BUFFER_LEN: usize = 4096;

/// A spawned task handle.
#[derive(Debug)]
pub struct TaskHandle(Receiver<Result<NonEmpty<ExecutionResult>, backend::TaskRunError>>);

impl TaskHandle {
    /// Consumes the task handle and waits for the task to complete.
    ///
    /// Returns the exit statuses of the task's executors.
    pub async fn wait(self) -> Result<NonEmpty<ExecutionResult>, backend::TaskRunError> {
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
    ) -> Result<Self> {
        let names = Arc::new(Mutex::new(GeneratorIterator::new(
            UniqueAlphanumeric::default_with_expected_generations(NAME_BUFFER_LEN),
            NAME_BUFFER_LEN,
        )));

        let backend = match config {
            Kind::Docker(config) => {
                let backend = docker::Backend::initialize_default_with(config, names).await?;
                Arc::new(backend) as Arc<dyn Backend>
            }
            Kind::Generic(config) => {
                let backend = generic::Backend::initialize(config, defaults, names).await?;
                Arc::new(backend)
            }
            Kind::TES(config) => Arc::new(tes::Backend::initialize(config, names).await),
        };

        Ok(Self {
            backend,
            lock: Arc::new(Semaphore::new(max_tasks)),
        })
    }

    /// Spawns a task to be executed by the backend.
    ///
    /// The optional `events` parameter is used to broadcast Crankshaft events
    /// for the task's execution. Events are not broadcast when passed a `None`.
    ///
    /// The `token` parameter is a cancellation token that can be used to cancel
    /// the task's execution.
    pub async fn spawn(
        &self,
        task: Task,
        events: Option<broadcast::Sender<Event>>,
        token: CancellationToken,
    ) -> anyhow::Result<TaskHandle> {
        trace!(backend = ?self.backend, task = ?task);

        let (tx, rx) = tokio::sync::oneshot::channel();
        let backend = self.backend.clone();
        let lock = self.lock.clone();

        tokio::spawn(async move {
            let _permit = lock.acquire().await?;
            let result = backend.clone().run(task, events, token)?.await;

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
