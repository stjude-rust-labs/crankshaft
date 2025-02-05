//! Task outputs.

mod builder;

pub use builder::Builder;
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
#[derive(Clone, Debug)]
pub struct Output {
    /// An optional name.
    name: Option<String>,

    /// An optional description.
    description: Option<String>,

    /// The URL to copy the output to when complete.
    url: Url,

    /// The path to map the output to within the container.
    path: String,

    /// The type of the output.
    r#type: Type,
}

impl Output {
    /// Gets a new builder for an output.
    pub fn builder() -> Builder {
        Builder::default()
    }

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
    pub fn r#type(&self) -> &Type {
        &self.r#type
    }
}
