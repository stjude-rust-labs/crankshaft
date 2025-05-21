//! Supported backends.

use std::fmt::Debug;
use std::process::ExitStatus;

use anyhow::Result;
use async_trait::async_trait;
use futures::future::BoxFuture;
use nonempty::NonEmpty;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::Task;

pub mod docker;
pub mod generic;
pub mod tes;

/// Represents an error that may occur when running a task.
#[derive(Debug, thiserror::Error)]
pub enum TaskRunError {
    /// The task has been canceled.
    #[error("the task has been canceled")]
    Canceled,
    /// The task has been preempted.
    ///
    /// This error is only returned from backends that support preemptible
    /// tasks.
    #[error("the task has been preempted")]
    Preempted,
    /// Another error occurred while running the task.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// An execution backend.
#[async_trait]
pub trait Backend: Debug + Send + Sync + 'static {
    /// Gets the default name for the backend.
    fn default_name(&self) -> &'static str;

    /// Runs a task in a backend.
    ///
    /// The optional `started` channel is notified when the first execution of
    /// the task has started.
    ///
    /// Returns a collection of exit status corresponding to the task's
    /// executions.
    fn run(
        &self,
        task: Task,
        started: Option<oneshot::Sender<()>>,
        token: CancellationToken,
    ) -> Result<BoxFuture<'static, Result<NonEmpty<ExitStatus>, TaskRunError>>>;
}
