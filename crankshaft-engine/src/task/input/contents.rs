//! Contents of an input.

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

    /// Contents provided as a string literal.
    Literal(String),
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
    /// * The second value is the literal contents as a [`String`] if the type
    ///   is [`Contents::Literal`]. Else, the value is [`None`].
    pub fn one_hot(self) -> (Option<Url>, Option<String>) {
        match self {
            Contents::Url(url) => (Some(url), None),
            Contents::Literal(value) => (None, Some(value)),
        }
    }
}
