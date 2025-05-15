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
    ) -> Result<BoxFuture<'static, Result<NonEmpty<ExitStatus>>>>;
}
