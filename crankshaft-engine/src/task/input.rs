//! Task inputs.

use bon::Builder;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

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
    /// A name.
    #[builder(into)]
    name: Option<String>,

    /// A description.
    #[builder(into)]
    description: Option<String>,

    /// The contents.
    #[builder(into)]
    contents: Contents,

    /// The path to map the input to within the container.
    #[builder(into)]
    path: String,

    /// The type of the input.
    #[builder(into)]
    ty: Type,
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

    /// Fetches the file contents via an [`AsyncRead`]er.
    pub async fn fetch(&self) -> Vec<u8> {
        match &self.contents {
            Contents::Literal(content) => content.as_bytes().to_vec(),
            Contents::Url(url) => match url.scheme() {
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

impl From<Input> for tes::v1::types::task::Input {
    fn from(input: Input) -> Self {
        let Input {
            name,
            description,
            contents,
            path,
            ty,
        } = input;

        let (url, content) = contents.one_hot();

        let r#type = match ty {
            Type::File => tes::v1::types::task::file::Type::File,
            Type::Directory => tes::v1::types::task::file::Type::Directory,
        };

        tes::v1::types::task::Input {
            name,
            description,
            url: url.map(|url| url.to_string()),
            path,
            r#type,
            content,
        }
    }
}
