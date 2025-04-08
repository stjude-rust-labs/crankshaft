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
    pub name: Option<String>,

    /// A description of the input.
    #[builder(into)]
    pub description: Option<String>,

    /// The contents of the input.
    #[builder(into)]
    pub contents: Contents,

    /// The expected guest path of the input.
    #[builder(into)]
    pub path: String,

    /// The type of the input.
    #[builder(into)]
    pub ty: Type,

    /// Whether or not the input should be treated as read-only.
    ///
    /// Defaults to `true`.
    #[builder(default = true)]
    pub read_only: bool,
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

        let r#type = match ty {
            Type::File => tes::v1::types::task::file::Type::File,
            Type::Directory => tes::v1::types::task::file::Type::Directory,
        };

        Ok(tes::v1::types::task::Input {
            name,
            description,
            url: url.map(|url| url.to_string()),
            path,
            r#type,
            content: content
                .map(|v| String::from_utf8(v).context("TES requires file content to be UTF-8"))
                .transpose()?,
        })
    }
}
