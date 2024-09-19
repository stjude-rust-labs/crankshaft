//! Builders for an [`Execution`].

use indexmap::IndexMap;
use nonempty::NonEmpty;

use crate::task::Execution;

/// An error related to a [`Builder`].
#[derive(Debug)]
pub enum Error {
    /// A required value was missing for a builder field.
    Missing(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Missing(field) => write!(
                f,
                "missing required value for '{field}' in a task execution builder"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// A [`Result`](std::result::Result) with an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// A builder for a [`Execution`].
#[derive(Debug, Default)]
pub struct Builder {
    /// The container image.
    image: Option<String>,

    /// The command arguments to execute.
    args: Option<NonEmpty<String>>,

    /// The working directory, if configured.
    working_directory: Option<String>,

    /// The path inside the container to a file whose contents will be piped to
    /// the standard input, if configured.
    stdin: Option<String>,

    /// The path inside the container to a file where the contents of the
    /// standard output stream will be written, if configured.
    stdout: Option<String>,

    /// The path inside the container to a file where the contents of the
    /// standard error stream will be written, if configured.
    stderr: Option<String>,

    /// A map of environment variables, if configured.
    env: Option<IndexMap<String, String>>,
}

impl Builder {
    /// Adds an image to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous image(s) provided to the
    /// builder.
    pub fn image(mut self, image: impl Into<String>) -> Self {
        self.image = Some(image.into());
        self
    }

    /// Resets the args to [`None`].
    pub fn reset_args(mut self) -> Self {
        self.args = None;
        self
    }

    /// Adds args to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will append to any previously assigned args (use
    /// [`reset_args()`](Self::reset_args) if you need to erase the previously
    /// provided args).
    pub fn args(mut self, values: impl IntoIterator<Item: Into<String>>) -> Self {
        let mut values = values.into_iter().map(|s| s.into());

        self.args = match self.args {
            Some(mut args) => {
                args.extend(values);
                Some(args)
            }
            None => {
                if let Some(arg) = values.next() {
                    let mut args = NonEmpty::new(arg);
                    args.extend(values);
                    Some(args)
                } else {
                    None
                }
            }
        };

        self
    }

    /// Adds a working directory to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous working directories provided
    /// to the builder.
    pub fn working_directory(mut self, value: impl Into<String>) -> Self {
        self.working_directory = Some(value.into());
        self
    }

    /// Adds a file with which to stream standard in from.
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous standard in declarations
    /// provided to the builder.
    pub fn stdin(mut self, value: impl Into<String>) -> Self {
        self.stdin = Some(value.into());
        self
    }

    /// Adds a file with which to stream standard out to.
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous standard out declarations
    /// provided to the builder.
    pub fn stdout(mut self, stdout: impl Into<String>) -> Self {
        self.stdout = Some(stdout.into());
        self
    }

    /// Adds a file with which to stream standard error to.
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous standard error declarations
    /// provided to the builder.
    pub fn stderr(mut self, stderr: impl Into<String>) -> Self {
        self.stderr = Some(stderr.into());
        self
    }

    /// Adds an environment variable to the builder.
    ///
    /// # Notes
    ///
    /// If an environment variable is added more than once, the previous values
    /// will be overwritten by the last provided value.
    pub fn env(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let mut env = self.env.unwrap_or_default();
        env.insert(name.into(), value.into());
        self.env = Some(env);
        self
    }

    /// Consumes `self` and attempts to return a built [`Execution`].
    pub fn try_build(self) -> Result<Execution> {
        let image = self.image.map(Ok).unwrap_or(Err(Error::Missing("image")))?;
        let args = self.args.map(Ok).unwrap_or(Err(Error::Missing("args")))?;

        Ok(Execution {
            image,
            args,
            workdir: self.working_directory,
            stdin: self.stdin,
            stdout: self.stdout,
            stderr: self.stderr,
            env: self.env,
        })
    }
}
