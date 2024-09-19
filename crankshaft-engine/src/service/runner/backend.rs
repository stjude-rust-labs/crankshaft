//! Supported backends.

use std::fmt::Debug;
use std::process::Output;

use async_trait::async_trait;
use futures::future::BoxFuture;
use nonempty::NonEmpty;

use crate::Task;

pub mod docker;
pub mod generic;
pub mod tes;

/// A reply from a backend when a task is completed.
#[derive(Clone, Debug)]
pub struct TaskResult {
    /// The results from each execution.
    pub(crate) executions: NonEmpty<Output>,
}

impl TaskResult {
    /// Gets the execution results.
    pub fn executions(&self) -> &NonEmpty<Output> {
        &self.executions
    }
}

/// An execution backend.
#[async_trait]
pub trait Backend: Debug + Send + Sync + 'static {
    /// Gets the default name for the backend.
    fn default_name(&self) -> &'static str;

    /// Runs a task in a backend.
    fn run(&self, task: Task) -> BoxFuture<'static, TaskResult>;
}
