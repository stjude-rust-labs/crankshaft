//! Supported backends.

use std::fmt::Debug;
use std::process::ExitStatus;

use anyhow::Result;
use async_trait::async_trait;
use crankshaft_monitor::proto::Event;
use futures::future::BoxFuture;
use nonempty::NonEmpty;
use tokio::sync::broadcast;
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
    /// The optional event_sender sends the task lifecycle events to any client
    /// connected to `Crankshaft`
    ///
    /// Returns a collection of exit status corresponding to the task's
    /// executions.
    fn run(
        &self,
        task: Task,
        event_sender: Option<broadcast::Sender<Event>>,
        token: CancellationToken,
    ) -> Result<BoxFuture<'static, Result<NonEmpty<ExitStatus>, TaskRunError>>>;
}
