//! Task inputs.

use std::borrow::Cow;
use std::fs;

use bon::Builder;
use eyre::Context;
use eyre::Result;
use eyre::bail;
use eyre::eyre;
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
    /// An optional name to give the input.
    #[builder(into)]
    name: Option<String>,

    /// A description of the input.
    #[builder(into)]
    description: Option<String>,

    /// The contents of the input.
    #[builder(into)]
    contents: Contents,

    /// The expected guest path of the input.
    #[builder(into)]
    path: String,

    /// The type of the input.
    #[builder(into)]
    ty: Type,

    /// Whether or not the input should be treated as read-only.
    ///
    /// Defaults to `true`.
    #[builder(default = true)]
    read_only: bool,
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

    /// Fetches the input contents.
    ///
    /// This method will return an error if the input is a path to a directory.
    pub async fn fetch(&self) -> Result<Cow<'_, [u8]>> {
        match &self.contents {
            Contents::Literal(bytes) => Ok(bytes.into()),
            Contents::Url(url) => match url.scheme() {
                "file" => {
                    // SAFETY: we just checked to ensure this is a file, so
                    // getting the file path should always unwrap.
                    let path = url.to_file_path().map_err(|_| {
                        eyre!(
                            "URL `{url}` has a file scheme but cannot be represented as a file \
                             path"
                        )
                    })?;
                    let mut file = File::open(&path).await.with_context(|| {
                        format!("failed to open file `{path}`", path = path.display())
                    })?;
                    let mut buffer = Vec::with_capacity(4096);
                    file.read_to_end(&mut buffer).await.with_context(|| {
                        format!("failed to read file `{path}`", path = path.display())
                    })?;
                    Ok(buffer.into())
                }
                "http" | "https" => bail!("support for HTTP URLs is not yet implemented"),
                "s3" => bail!("support for S3 URLs is not yet implemented"),
                scheme => bail!("URL has unsupported scheme `{scheme}`"),
            },
            Contents::Path(path) => Ok(fs::read_to_string(path)
                .with_context(|| format!("failed to read file `{path}`", path = path.display()))?
                .into_bytes()
                .into()),
        }
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
