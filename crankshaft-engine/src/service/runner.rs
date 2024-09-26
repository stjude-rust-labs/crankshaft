//! Task runner services.

use std::sync::Arc;
use std::sync::Mutex;

use crankshaft_config::backend::Defaults;
use crankshaft_config::backend::Kind;
use futures::future::BoxFuture;
use futures::future::join_all;
use futures::stream::FuturesUnordered;
use tokio::sync::Semaphore;
use tokio::sync::oneshot::Receiver;
use tracing::trace;

pub mod backend;

pub use backend::Backend;

use super::name::GeneratorIterator;
use super::name::UniqueAlphanumeric;
use crate::Result;
use crate::Task;
use crate::service::runner::backend::TaskResult;
use crate::service::runner::backend::docker;
use crate::service::runner::backend::generic;
use crate::service::runner::backend::tes;

/// The size of the name buffer.
const NAME_BUFFER_LEN: usize = 4096;

/// A submitted task handle.
#[derive(Debug)]
pub struct TaskHandle {
    /// A callback that is executed when a task is completed.
    pub callback: Receiver<TaskResult>,
}

/// A generic task runner.
#[derive(Debug)]
pub struct Runner {
    /// The task runner itself.
    backend: Arc<dyn Backend>,

    /// The task lock.
    lock: Arc<tokio::sync::Semaphore>,

    /// The list of submitted tasks.
    pub tasks: FuturesUnordered<BoxFuture<'static, TaskResult>>,

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
                let backend = docker::Backend::initialize_default_with(config)?;
                Arc::new(backend) as Arc<dyn Backend>
            }
            Kind::Generic(config) => {
                let backend = generic::Backend::initialize(config, defaults).await?;
                Arc::new(backend)
            }
            Kind::TES(config) => Arc::new(tes::Backend::initialize(config)),
        };

        let generator = UniqueAlphanumeric::default_with_expected_generations(max_tasks);

        Ok(Self {
            backend,
            lock: Arc::new(Semaphore::new(max_tasks)),
            tasks: Default::default(),
            name_generator: Arc::new(Mutex::new(GeneratorIterator::new(
                generator,
                NAME_BUFFER_LEN,
            ))),
        })
    }

    /// Submits a task to be executed by the backend.
    pub fn submit(&self, mut task: Task) -> TaskHandle {
        trace!(backend = ?self.backend, task = ?task);

        let (tx, rx) = tokio::sync::oneshot::channel();
        let backend = self.backend.clone();
        let lock = self.lock.clone();

        if backend.default_name() == "docker" && task.name().is_none() {
            let mut generator = self.name_generator.lock().unwrap();
            // SAFETY: this generator should _never_ run out of entries.
            task.set_name(generator.next().unwrap());
        }

        let fun = async move {
            let _permit = lock.acquire().await;

            let result = backend.clone().run(task).await;

            // NOTE: if the send does not succeed, that is almost certainly
            // because the receiver was dropped. That is a relatively standard
            // practice if you don't specifically _want_ to keep a handle to the
            // returned result, so we ignore any errors related to that.
            let _ = tx.send(result.clone());
            drop(_permit);

            result
        };

        self.tasks.push(Box::pin(fun));
        TaskHandle { callback: rx }
    }

    /// Gets the tasks from the runner.
    pub fn tasks(self) -> impl Iterator<Item = BoxFuture<'static, TaskResult>> {
        self.tasks.into_iter()
    }

    /// Runs all of the tasks scheduled in the [`Runner`].
    pub async fn run(self) {
        join_all(self.tasks).await;
    }
}
