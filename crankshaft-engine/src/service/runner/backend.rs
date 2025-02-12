//! Supported backends.

use std::fmt::Debug;
use std::process::Output;
use std::sync::Arc;

use async_trait::async_trait;
use eyre::Result;
use futures::future::BoxFuture;
use nonempty::NonEmpty;
use tokio_util::sync::CancellationToken;

use crate::Task;

pub mod docker;
pub mod generic;
pub mod tes;

/// An execution backend.
#[async_trait]
pub trait Backend: Debug + Send + Sync + 'static {
    /// Gets the default name for the backend.
    fn default_name(&self) -> &'static str;

    /// Runs a task in a backend.
    ///
    /// The `started` callback is called when an execution of the task has
    /// started; the parameter is the index of the execution into the task's
    /// executions collection.
    // TODO: use a representation of task output that isn't based on
    // `std::process::Output` that would allow us to write stdout/stderror to a file
    // instead of buffering it all in memory
    fn run(
        &self,
        task: Task,
        started: Arc<dyn Fn(usize) + Send + Sync + 'static>,
        token: CancellationToken,
    ) -> Result<BoxFuture<'static, Result<NonEmpty<Output>>>>;
}
