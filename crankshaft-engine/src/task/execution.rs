//! A unit of executable work.

use std::hash::RandomState;

use bon::Builder;
use indexmap::IndexMap;
use nonempty::NonEmpty;

/// An execution.
#[derive(Builder, Clone, Debug)]
#[builder(builder_type = Builder)]
pub struct Execution {
    /// The container image.
    #[builder(into)]
    image: String,

    /// The command arguments to execute.
    #[builder(into)]
    args: NonEmpty<String>,

    /// The working directory, if configured.
    #[builder(into)]
    workdir: Option<String>,

    /// The path inside the container to a file whose contents will be piped to
    /// the standard input, if configured.
    #[builder(into)]
    stdin: Option<String>,

    /// The path inside the container to a file where the contents of the
    /// standard output stream will be written, if configured.
    #[builder(into)]
    stdout: Option<String>,

    /// The path inside the container to a file where the contents of the
    /// standard error stream will be written, if configured.
    #[builder(into)]
    stderr: Option<String>,

    /// A map of environment variables, if configured.
    #[builder(into)]
    env: Option<IndexMap<String, String>>,
}

impl Execution {
    /// The image for the execution to run within.
    pub fn image(&self) -> &str {
        &self.image
    }

    /// The arguments to the execution.
    pub fn args(&self) -> &NonEmpty<String> {
        &self.args
    }

    /// The working directory.
    pub fn workdir(&self) -> Option<&String> {
        self.workdir.as_ref()
    }

    /// The file to pipe the standard input stream from.
    pub fn stdin(&self) -> Option<&String> {
        self.stdin.as_ref()
    }

    /// The file to pipe the standard output stream to.
    pub fn stdout(&self) -> Option<&String> {
        self.stdout.as_ref()
    }

    /// The file to pipe the standard error stream to.
    pub fn stderr(&self) -> Option<&String> {
        self.stderr.as_ref()
    }

    /// The environment variables for the execution.
    pub fn env(&self) -> Option<&IndexMap<String, String, RandomState>> {
        self.env.as_ref()
    }
}
