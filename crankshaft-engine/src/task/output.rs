//! Task outputs.

use bon::Builder;
use url::Url;

/// A type of task output.
#[derive(Clone, Debug)]
pub enum Type {
    /// A file.
    File,

    /// A directory.
    Directory,
}

/// A task output.
#[derive(Builder, Clone, Debug)]
#[builder(builder_type = Builder)]
pub struct Output {
    /// An optional name.
    #[builder(into)]
    pub name: Option<String>,

    /// An optional description.
    #[builder(into)]
    pub description: Option<String>,

    /// The URL to copy the output to when complete.
    #[builder(into)]
    pub url: Url,

    /// The path to map the output to within the container.
    #[builder(into)]
    pub path: String,

    /// The type of the output.
    #[builder(into)]
    pub ty: Type,
}

impl From<Output> for tes::v1::types::task::Output {
    fn from(output: Output) -> Self {
        let Output {
            name,
            description,
            url,
            path,
            ty,
        } = output;

        let r#type = match ty {
            Type::File => tes::v1::types::task::file::Type::File,
            Type::Directory => tes::v1::types::task::file::Type::Directory,
        };

        tes::v1::types::task::Output {
            name,
            description,
            url: url.to_string(),
            path,
            r#type,
        }
    }
}
