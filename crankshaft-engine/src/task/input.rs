//! Task inputs.

mod builder;

use std::path::PathBuf;

pub use builder::Builder;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use url::Url;

/// A type of input.
#[derive(Clone, Debug)]
pub enum Type {
    /// A file.
    File,

    /// A directory.
    Directory,
}

/// The source of an input.
#[derive(Clone, Debug)]
pub enum Contents {
    /// Contents sourced from a URL.
    URL(Url),

    /// Contents provided as a string literal.
    Literal(String),
}

impl From<PathBuf> for Contents {
    fn from(value: PathBuf) -> Self {
        let url = Url::from_file_path(value).unwrap_or_else(|_| panic!("Invalid path"));
        Contents::URL(url)
    }
}

/// An input to a task.
#[derive(Clone, Debug)]
pub struct Input {
    /// A name.
    name: Option<String>,

    /// A description.
    description: Option<String>,

    /// The contents.
    contents: Contents,

    /// The path to map the input to within the container.
    path: String,

    /// The type of the input.
    r#type: Type,
}

impl Input {
    /// Gets a new builder for an input.
    pub fn builder() -> Builder {
        Builder::default()
    }

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
    pub fn r#type(&self) -> &Type {
        &self.r#type
    }

    /// Fetches the file contents via an [`AsyncRead`]er.
    pub async fn fetch(&self) -> Vec<u8> {
        match &self.contents {
            Contents::Literal(content) => content.as_bytes().to_vec(),
            Contents::URL(url) => match url.scheme() {
                "file" => {
                    // SAFETY: we just checked to ensure this is a file, so
                    // getting the file path should always unwrap.
                    let path = url.to_file_path().unwrap();
                    let mut file = File::open(path).await.unwrap();
                    let mut buffer = Vec::with_capacity(4096);
                    file.read_to_end(&mut buffer).await.unwrap();
                    buffer
                }
                "http" | "https" => unimplemented!("http(s) URL support not implemented"),
                "s3" => unimplemented!("s3 URL support not implemented"),
                v => unreachable!("unsupported URL scheme: {v}"),
            },
        }
    }
}
