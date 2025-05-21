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
    pub(crate) name: Option<String>,

    /// An optional description.
    #[builder(into)]
    pub(crate) description: Option<String>,

    /// The URL to copy the output to when complete.
    #[builder(into)]
    pub(crate) url: Url,

    /// The path to map the output to within the container.
    #[builder(into)]
    pub(crate) path: String,

    /// The type of the output.
    #[builder(into)]
    pub(crate) ty: Type,
}

impl Output {
    /// The name of the output (if it exists).
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The description of the output (if it exists).
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The URL to place the file when the task completes.
    pub fn url(&self) -> &str {
        self.url.as_ref()
    }

    /// The path to the file within the container.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The type of the output.
    pub fn ty(&self) -> &Type {
        &self.ty
    }
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

        let ty = match ty {
            Type::File => tes::v1::types::task::IoType::File,
            Type::Directory => tes::v1::types::task::IoType::Directory,
        };

        Self {
            name,
            description,
            url: url.to_string(),
            path,
            path_prefix: None,
            ty,
        }
    }
}
