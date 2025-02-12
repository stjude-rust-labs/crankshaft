//! Task runner services.

use std::process::Output;
use std::sync::Arc;
use std::sync::Mutex;

use crankshaft_config::backend::Defaults;
use crankshaft_config::backend::Kind;
use nonempty::NonEmpty;
use tokio::sync::Semaphore;
use tokio::sync::oneshot::Receiver;
use tokio_util::sync::CancellationToken;
use tracing::trace;

pub mod backend;

pub use backend::Backend;

use crate::Result;
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
pub struct TaskHandle(Receiver<Result<NonEmpty<Output>>>);

impl TaskHandle {
    /// Consumes the task handle and waits for the task to complete.
    ///
    /// Returns the output of the task.
    pub async fn wait(self) -> Result<NonEmpty<Output>> {
        self.0.await?
    }
}

/// A generic task runner.
#[derive(Debug)]
pub struct Runner {
    /// The task runner itself.
    backend: Arc<dyn Backend>,

    /// The task lock.
    lock: Arc<tokio::sync::Semaphore>,

    /// The unique name generator for tasks without names being sent to backends
    /// that may need names.
    name_generator: Arc<Mutex<GeneratorIterator<UniqueAlphanumeric>>>,
}

impl Runner {
    /// Creates a new [`Runner`].
    pub async fn initialize(
        config: Kind,
        max_tasks: usize,
        defaults: Option<Defaults>,
    ) -> Result<Self> {
        let backend = match config {
            Kind::Docker(config) => {
                let backend = docker::Backend::initialize_default_with(config).await?;
                Arc::new(backend) as Arc<dyn Backend>
            }
            Kind::Generic(config) => {
                let backend = generic::Backend::initialize(config, defaults).await?;
                Arc::new(backend)
            }
            Kind::TES(config) => Arc::new(tes::Backend::initialize(config)),
        };

        let generator = UniqueAlphanumeric::default_with_expected_generations(NAME_BUFFER_LEN);

        Ok(Self {
            backend,
            lock: Arc::new(Semaphore::new(max_tasks)),
            name_generator: Arc::new(Mutex::new(GeneratorIterator::new(
                generator,
                NAME_BUFFER_LEN,
            ))),
        })
    }

    /// Spawns a task to be executed by the backend.
    ///
    /// The `started` callback is called for each execution of the task that has
    /// started; the parameter is the index of the execution from the task's
    /// executions collection.
    ///
    /// The `cancellation` token can be used to gracefully cancel the task.
    pub fn spawn(&self, mut task: Task, token: CancellationToken) -> eyre::Result<TaskHandle> {
        trace!(backend = ?self.backend, task = ?task);

        let (tx, rx) = tokio::sync::oneshot::channel();
        let backend = self.backend.clone();
        let lock = self.lock.clone();

        if backend.default_name() == "docker" && task.name().is_none() {
            let mut generator = self.name_generator.lock().unwrap();
            // SAFETY: this generator should _never_ run out of entries.
            task.override_name(generator.next().unwrap());
        }

        tokio::spawn(async move {
            let _permit = lock.acquire().await?;
            let result = backend.clone().run(task, Arc::new(|_| {}), token)?.await;

            // NOTE: if the send does not succeed, that is almost certainly
            // because the receiver was dropped. That is a relatively standard
            // practice if you don't specifically _want_ to keep a handle to the
            // returned result, so we ignore any errors related to that.
            let _ = tx.send(result);
            drop(_permit);

            eyre::Ok(())
        });

        Ok(TaskHandle(rx))
    }
}
