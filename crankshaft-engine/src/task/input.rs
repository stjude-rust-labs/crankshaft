//! Task inputs.

use bon::Builder;
use eyre::Context;
use eyre::Result;

mod contents;

pub use contents::Contents;

/// A type of input.
#[derive(Clone, Debug)]
pub enum Type {
    /// A file.
    File,

    /// A directory.
    Directory,
}

/// An input to a task.
#[derive(Builder, Clone, Debug)]
#[builder(builder_type = Builder)]
pub struct Input {
    /// An optional name to give the input.
    #[builder(into)]
    pub(crate) name: Option<String>,

    /// A description of the input.
    #[builder(into)]
    pub(crate) description: Option<String>,

    /// The contents of the input.
    #[builder(into)]
    pub(crate) contents: Contents,

    /// The expected guest path of the input.
    #[builder(into)]
    pub(crate) path: String,

    /// The type of the input.
    #[builder(into)]
    pub(crate) ty: Type,

    /// Whether or not the input should be treated as read-only.
    ///
    /// Defaults to `true`.
    #[builder(default = true)]
    pub(crate) read_only: bool,
}

impl Input {
    /// The name of the input (if it exists).
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The description of the input (if it exists).
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The contents of the input.
    pub fn contents(&self) -> &Contents {
        &self.contents
    }

    /// The path where the input should be placed within the container.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The type of the container.
    pub fn ty(&self) -> &Type {
        &self.ty
    }

    /// Gets whether or not the input is read-only.
    ///
    /// Inputs are read-only by default.
    pub fn read_only(&self) -> bool {
        self.read_only
    }
}

impl TryFrom<Input> for tes::v1::types::task::Input {
    type Error = eyre::Error;

    fn try_from(input: Input) -> Result<Self, Self::Error> {
        let Input {
            name,
            description,
            contents,
            path,
            ty,
            read_only: _,
        } = input;

        let (url, content) = contents.one_hot()?;

        let ty = match ty {
            Type::File => tes::v1::types::task::IoType::File,
            Type::Directory => tes::v1::types::task::IoType::Directory,
        };

        Ok(tes::v1::types::task::Input {
            name,
            description,
            url: url.map(|url| url.to_string()),
            path,
            ty,
            content: content
                .map(|v| String::from_utf8(v).context("TES requires file content to be UTF-8"))
                .transpose()?,
            streamable: None,
        })
    }
}
