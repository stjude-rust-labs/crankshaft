//! Supported backends.

use std::fmt::Debug;
use std::process::Output;

use async_trait::async_trait;
use eyre::Result;
use futures::future::BoxFuture;
use nonempty::NonEmpty;

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
    // TODO: use a representation of task output that isn't based on
    // `std::process::Output` that would allow us to write stdout/stderror to a file
    // instead of buffering it all in memory
    fn run(&self, task: Task) -> Result<BoxFuture<'static, Result<NonEmpty<Output>>>>;
}
