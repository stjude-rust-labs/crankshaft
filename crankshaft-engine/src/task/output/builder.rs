//! Builders for an [`Output`].

use url::Url;

use crate::task::Output;
use crate::task::output::Type;

/// An error related to a [`Builder`].
#[derive(Debug)]
pub enum Error {
    /// A required value was missing for a builder field.
    Missing(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Missing(field) => {
                write!(
                    f,
                    "missing required value for '{field}' in a task output builder"
                )
            }
        }
    }
}

impl std::error::Error for Error {}

/// A [`Result`](std::result::Result) with an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// A builder for a [`Output`].
#[derive(Debug, Default)]
pub struct Builder {
    /// An optional name.
    name: Option<String>,

    /// An optional description.
    description: Option<String>,

    /// The URL to copy the output to when complete.
    url: Option<Url>,

    /// The path to map the output to within the container.
    path: Option<String>,

    /// The type of the output.
    r#type: Option<Type>,
}

impl Builder {
    /// Adds a name to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous name(s) provided to the
    /// builder.
    pub fn name(mut self, value: impl Into<String>) -> Self {
        self.name = Some(value.into());
        self
    }

    /// Adds a description to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous description(s) provided to the
    /// builder.
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }

    /// Adds a URL to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous url(s) provided to the
    /// builder.
    pub fn url(mut self, value: impl Into<Url>) -> Self {
        self.url = Some(value.into());
        self
    }

    /// Adds a path to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous path(s) provided to the
    /// builder.
    pub fn path(mut self, value: impl Into<String>) -> Self {
        self.path = Some(value.into());
        self
    }

    /// Adds a type to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous type(s) provided to the
    /// builder.
    pub fn r#type(mut self, value: impl Into<Type>) -> Self {
        self.r#type = Some(value.into());
        self
    }

    /// Consumes `self` and attempts to return a built [`Output`].
    pub fn try_build(self) -> Result<Output> {
        let url = self.url.ok_or(Error::Missing("url"))?;
        let path = self.path.ok_or(Error::Missing("path"))?;
        let r#type = self.r#type.ok_or(Error::Missing("type"))?;

        Ok(Output {
            name: self.name,
            description: self.description,
            url,
            path,
            r#type,
        })
    }
}
