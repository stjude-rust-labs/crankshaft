//! Configuration related to the _TES_ execution backend.

mod builder;
pub mod http;

pub use builder::Builder;
use serde::Deserialize;
use serde::Serialize;
use url::Url;

/// A configuration object for a TES execution backend.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// The URL to reach the TES service at.
    url: Url,

    /// More nuanced, HTTP-related configuration.
    http: http::Config,
}

impl Config {
    /// Gets a builder for [`Config`].
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Gets the URL of the TES server.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Gets the HTTP-related configuration.
    pub fn http(&self) -> &http::Config {
        &self.http
    }
}
