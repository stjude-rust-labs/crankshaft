//! Configuration related to the _TES_ execution backend.

pub mod http;

use bon::Builder;
use serde::Deserialize;
use serde::Serialize;
use url::Url;

/// A configuration object for a TES execution backend.
#[derive(Builder, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[builder(builder_type = Builder)]
pub struct Config {
    /// The URL to reach the TES service at.
    #[builder(into)]
    url: Url,

    /// More nuanced, HTTP-related configuration.
    #[builder(into)]
    http: http::Config,
}

impl Config {
    /// Gets the URL of the TES server.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Gets the HTTP-related configuration.
    pub fn http(&self) -> &http::Config {
        &self.http
    }
}
