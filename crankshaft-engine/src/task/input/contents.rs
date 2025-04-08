//! Contents of an input.

use std::borrow::Cow;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use eyre::Context;
use eyre::bail;
use eyre::eyre;
use thiserror::Error;
use url::Url;

/// An error related to an input's [`Contents`].
#[derive(Error, Debug)]
pub enum Error {
    /// An error parsing a [`Url`](url::Url).
    #[error("invalid URL: {0}")]
    ParseUrl(url::ParseError),
}

/// A [`Result`](std::result::Result) with an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// The source of an input.
#[derive(Clone, Debug)]
pub enum Contents {
    /// Contents sourced from a URL.
    Url(Url),

    /// Contents provided as a literal array of bytes.
    Literal(Vec<u8>),

    /// Contents are provided as a path to a file or directory on the host
    /// system.
    Path(PathBuf),
}

impl Contents {
    /// Attempts to create a URL contents from a string slice.
    pub fn url_from_str(url: impl AsRef<str>) -> Result<Self> {
        url.as_ref().parse().map(Self::Url).map_err(Error::ParseUrl)
    }

    /// Consumes `self` and one hot encodes the inner contents.
    ///
    /// * The first value is the [`Url`] if the type is [`Contents::Url`]. Else,
    ///   the value is [`None`].
    /// * The second value is the literal contents as a [`Vec<u8>`] if the type
    ///   is [`Contents::Literal`] or [`Contents::Path`]. Else, the value is
    ///   [`None`].
    ///
    /// Returns an error if the contents are to a path and the file contents
    /// could not be read.
    pub fn one_hot(self) -> eyre::Result<(Option<Url>, Option<Vec<u8>>)> {
        match self {
            Self::Url(url) => Ok((Some(url), None)),
            Self::Literal(value) => Ok((None, Some(value))),
            Self::Path(path) => Ok((
                None,
                Some(fs::read(&path).with_context(|| {
                    format!("failed to read file `{path}`", path = path.display())
                })?),
            )),
        }
    }

    /// Fetches the contents locally.
    ///
    /// If the contents is a path, the path is returned.
    ///
    /// If the contents is a literal, they are written to a temporary file.
    ///
    /// If the contents is a URL, the file is downloaded to a temporary file.
    ///
    /// Returns the path to the contents.
    pub async fn fetch(&self, temp_dir: &Path) -> eyre::Result<Cow<'_, Path>> {
        let contents: Cow<'_, [u8]> = match self {
            Self::Url(url) => {
                match url.scheme() {
                    "file" => {
                        // SAFETY: we just checked to ensure this is a file, so
                        // getting the file path should always unwrap.
                        let path = url.to_file_path().map_err(|_| {
                            eyre!(
                                "URL `{url}` has a file scheme but cannot be represented as a \
                                 file path"
                            )
                        })?;
                        return Ok(path.into());
                    }
                    // TODO: remotely fetched contents should be cached somewhere
                    "http" | "https" => bail!("support for HTTP URLs is not yet implemented"),
                    "s3" => bail!("support for S3 URLs is not yet implemented"),
                    "az" => bail!("support for Azure Storage URLs is not yet implemented"),
                    "gs" => bail!("support for Google Cloud Storage URLs is not yet implemented"),
                    scheme => bail!("URL has unsupported scheme `{scheme}`"),
                }
            }
            Self::Literal(bytes) => bytes.into(),
            Self::Path(path) => return Ok(path.into()),
        };

        // Write the contents to a temporary file within the given temporary directory
        let mut file = tempfile::NamedTempFile::new_in(temp_dir).with_context(|| {
            format!(
                "failed to create temporary input file in `{temp_dir}`",
                temp_dir = temp_dir.display()
            )
        })?;

        file.write(&contents).with_context(|| {
            format!(
                "failed to write input file contents to `{path}`",
                path = file.path().display()
            )
        })?;

        // Keep the file as the temporary directory itself will clean up the mounts
        let (_, path) = file.keep().context("failed to persist temporary file")?;

        Ok(path.into())
    }
}
