//! Builders for [execution backends](Config).

use crate::backend::Config;
use crate::backend::Defaults;
use crate::backend::Kind;

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
                "missing required value for '{field}' in the backend configuration builder"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// A [`Result`](std::result::Result) with an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// A builder for a [execution backend configuration object](Config).
#[derive(Default)]
pub struct Builder {
    /// The name.
    name: Option<String>,

    /// The kind.
    kind: Option<Kind>,

    /// The maximum number of concurrent tasks that can run.
    max_tasks: Option<usize>,

    /// The execution defaults.
    defaults: Option<Defaults>,
}

impl Builder {
    /// Sets the name for the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous names set within the builder.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the backend kind for the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous backend kinds set within the
    /// builder.
    pub fn kind(mut self, kind: impl Into<Kind>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// Sets the maximum number of tasks for the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous maximum number of tasks set
    /// within the builder.
    pub fn max_tasks(mut self, tasks: usize) -> Self {
        self.max_tasks = Some(tasks);
        self
    }

    /// Sets the execution defaults for the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous execution defaults set within
    /// the builder.
    pub fn defaults(mut self, defaults: impl Into<Defaults>) -> Self {
        self.defaults = Some(defaults.into());
        self
    }

    /// Consumes `self` and attempts to build a [`Config`].
    pub fn try_build(self) -> Result<Config> {
        let name = self.name.ok_or(Error::Missing("name"))?;
        let kind = self.kind.ok_or(Error::Missing("kind"))?;
        let max_tasks = self.max_tasks.ok_or(Error::Missing("max_tasks"))?;

        Ok(Config {
            name,
            kind,
            max_tasks,
            defaults: self.defaults,
        })
    }
}
