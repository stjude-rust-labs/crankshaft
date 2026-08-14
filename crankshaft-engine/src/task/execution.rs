//! A unit of executable work.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::process::ExitStatus;

use bon::Builder;
use indexmap::IndexMap;
use nonempty::NonEmpty;

/// An error used in [`Builder::images()`] when no images are specified.
#[derive(Debug)]
pub struct NoImageError;

impl Display for NoImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no image specified")
    }
}

impl std::error::Error for NoImageError {}

/// An execution.
#[derive(Builder, Clone, Debug)]
#[builder(builder_type = Builder)]
pub struct Execution {
    /// The container images.
    ///
    /// For backends that support it, multiple images can be specified to act as
    /// fallbacks in the event that the previous fails to pull.
    ///
    /// NOTE: Images will be tried in the order provided.
    #[builder(with = |iter: impl IntoIterator<Item = impl Into<String>>| -> Result<_, NoImageError> {
        NonEmpty::collect(iter.into_iter().map(Into::into)).ok_or(NoImageError)
    })]
    pub(crate) images: NonEmpty<String>,

    /// The program to execute.
    #[builder(into)]
    pub(crate) program: String,

    /// The arguments to the program.
    #[builder(into, default)]
    pub(crate) args: Vec<String>,

    /// The working directory, if configured.
    #[builder(into)]
    pub(crate) work_dir: Option<String>,

    /// The path inside the container to a file whose contents will be piped to
    /// the standard input, if configured.
    #[builder(into)]
    pub(crate) stdin: Option<String>,

    /// The path inside the container to a file where the contents of the
    /// standard output stream will be written, if configured.
    #[builder(into)]
    pub(crate) stdout: Option<String>,

    /// The path inside the container to a file where the contents of the
    /// standard error stream will be written, if configured.
    #[builder(into)]
    pub(crate) stderr: Option<String>,

    /// A map of environment variables, if configured.
    #[builder(into, default)]
    pub(crate) env: IndexMap<String, String>,
}

impl Execution {
    /// The images for the execution to run within.
    pub fn images(&self) -> &NonEmpty<String> {
        &self.images
    }

    /// The program to execute.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The arguments to the execution.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// The working directory.
    pub fn work_dir(&self) -> Option<&str> {
        self.work_dir.as_deref()
    }

    /// The file to pipe the standard input stream from.
    pub fn stdin(&self) -> Option<&str> {
        self.stdin.as_deref()
    }

    /// The file to pipe the standard output stream to.
    pub fn stdout(&self) -> Option<&str> {
        self.stdout.as_deref()
    }

    /// The file to pipe the standard error stream to.
    pub fn stderr(&self) -> Option<&str> {
        self.stderr.as_deref()
    }

    /// The environment variables for the execution.
    pub fn env(&self) -> &IndexMap<String, String> {
        &self.env
    }
}

impl From<Execution> for tes::v1::types::task::Executor {
    fn from(execution: Execution) -> Self {
        let env = execution
            .env
            .into_iter()
            .collect::<BTreeMap<String, String>>();

        let env = if env.is_empty() { None } else { Some(env) };

        let mut command = Vec::with_capacity(execution.args.len() + 1);
        command.push(execution.program);
        command.extend(execution.args);

        tes::v1::types::task::Executor {
            image: execution.images.first().into(),
            command,
            workdir: execution.work_dir,
            stdin: execution.stdin,
            stdout: execution.stdout,
            stderr: execution.stderr,
            env,
            ignore_error: Some(true),
        }
    }
}

/// The result of an [`Execution`].
#[derive(Clone, Debug)]
pub struct ExecutionResult {
    /// The name of the container image that was used in this execution.
    ///
    /// NOTE: While [`Execution`]s require an image, a backend is not
    /// necessarily required to make use of it.
    pub image: Option<String>,
    /// The exit status of the execution.
    pub status: ExitStatus,
}
