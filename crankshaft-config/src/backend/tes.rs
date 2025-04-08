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
    pub url: Url,

    /// More nuanced, HTTP-related configuration.
    #[builder(into, default)]
    pub http: http::Config,

    /// The poll interval, in seconds, to use for querying TES task status.
    ///
    /// Defaults to 1 second.
    pub interval: Option<u64>,
}
