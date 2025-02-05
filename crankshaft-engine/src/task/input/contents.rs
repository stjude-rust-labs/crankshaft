//! Contents of an input.

use url::Url;

/// An error related to an input's [`Contents`].
#[derive(Debug)]
pub enum Error {
    /// An error parsing a [`Url`](url::Url).
    ParseUrl(url::ParseError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ParseUrl(err) => write!(f, "parse url error: {err}"),
        }
    }
}

impl std::error::Error for Error {}

/// A [`Result`](std::result::Result) with an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// The source of an input.
#[derive(Clone, Debug)]
pub enum Contents {
    /// Contents sourced from a URL.
    Url(Url),

    /// Contents provided as a string literal.
    Literal(String),
}

impl Contents {
    /// Attempts to create a URL contents from a string slice.
    pub fn url_from_str(url: impl AsRef<str>) -> Result<Self> {
        url.as_ref().parse().map(Self::Url).map_err(Error::ParseUrl)
    }
}
