//! Contents of an input.

use std::fs;
use std::path::PathBuf;

use eyre::Context;
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
}
