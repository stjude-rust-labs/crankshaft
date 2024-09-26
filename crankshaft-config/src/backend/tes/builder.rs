//! Builders for the [_TES_ execution backend configuration](Config).

use url::Url;

use crate::backend::tes::Config;
use crate::backend::tes::http;

/// An error related to a [`Builder`].
#[derive(Debug)]
pub enum Error {
    /// A required value was missing for a builder field.
    Missing(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Missing(field) => write!(
                f,
                "missing required value for '{field}' in the TES backend configuration builder"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// A [`Result`](std::result::Result) with an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// A builder for the [TES execution backend configuration object](Config).
#[derive(Default)]
pub struct Builder {
    /// The URL to reach the TES service at.
    url: Option<Url>,

    /// More nuanced, HTTP-related configuration.
    http: Option<http::Config>,
}

impl Builder {
    /// Sets the URL for the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous URLs set within the builder.
    pub fn url(mut self, url: impl Into<Url>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the basic auth token for the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous basic auth tokens set within
    /// the builder.
    pub fn basic_auth_token(mut self, token: impl Into<String>) -> Self {
        let mut http = self.http.unwrap_or_default();
        http.basic_auth_token = Some(token.into());
        self.http = Some(http);
        self
    }

    /// Sets the HTTP-related configuration for the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous HTTP configuration set within
    /// the builder.
    pub fn http(mut self, http: impl Into<http::Config>) -> Self {
        self.http = Some(http.into());
        self
    }

    /// Consumes `self` and returns a built [`Config`].
    pub fn try_build(self) -> Result<Config> {
        let url = self.url.ok_or(Error::Missing("url"))?;
        let http = self.http.ok_or(Error::Missing("http"))?;

        Ok(Config { url, http })
    }
}
