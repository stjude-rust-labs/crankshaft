//! Builders for an [`Input`].

use crate::task::input::Contents;
use crate::task::input::Type;
use crate::task::Input;

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
                    "missing required value for '{field}' in a task input builder"
                )
            }
        }
    }
}

impl std::error::Error for Error {}

/// A [`Result`](std::result::Result) with an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// A builder for a [`Input`].
#[derive(Debug, Default)]
pub struct Builder {
    /// An optional name.
    name: Option<String>,

    /// An optional description.
    description: Option<String>,

    /// The input's contents.
    contents: Option<Contents>,

    /// The path to map the input to within the container.
    path: Option<String>,

    /// The type of the input.
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

    /// Adds contents to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous contents provided to the
    /// builder.
    pub fn contents(mut self, value: impl Into<Contents>) -> Self {
        self.contents = Some(value.into());
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

    /// Consumes `self` and attempts to return a built [`Input`].
    pub fn try_build(self) -> Result<Input> {
        let contents = self.contents.ok_or(Error::Missing("contents"))?;
        let path = self.path.ok_or(Error::Missing("path"))?;
        let r#type = self.r#type.ok_or(Error::Missing("type"))?;

        Ok(Input {
            name: self.name,
            description: self.description,
            contents,
            path,
            r#type,
        })
    }
}
