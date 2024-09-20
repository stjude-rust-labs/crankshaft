//! Task runner services.

use std::sync::Arc;

use crankshaft_config::backend::Defaults;
use crankshaft_config::backend::Kind;
use futures::future::join_all;
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use tokio::sync::oneshot::Receiver;
use tokio::sync::Semaphore;
use tracing::trace;

pub mod backend;

pub use backend::Backend;

use crate::service::runner::backend::docker;
use crate::service::runner::backend::generic;
use crate::service::runner::backend::tes;
use crate::service::runner::backend::TaskResult;
use crate::Result;
use crate::Task;

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

        Ok(Self {
            backend,
            lock: Arc::new(Semaphore::new(max_tasks)),
            tasks: Default::default(),
        })
    }

    /// Submits a task to be executed by the backend.
    pub fn submit(&self, task: Task) -> TaskHandle {
        trace!(backend = ?self.backend, task = ?task);

        let (tx, rx) = tokio::sync::oneshot::channel();
        let backend = self.backend.clone();
        let lock = self.lock.clone();

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
