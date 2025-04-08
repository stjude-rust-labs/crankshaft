//! Configuration related to an SSH-based command drivers.

use bon::Builder;
use serde::Deserialize;
use serde::Serialize;

/// Configuration related to SSH.
#[derive(Builder, Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[builder(builder_type = Builder)]
pub struct Config {
    /// A username.
    #[builder(into)]
    pub username: Option<String>,

    /// A port.
    pub port: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            username: Default::default(),
            port: 22,
        }
    }
}
