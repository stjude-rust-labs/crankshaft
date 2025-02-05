//! Configuration related to an SSH-based command drivers.

use bon::Builder;
use serde::Deserialize;
use serde::Serialize;

/// Configuration related to SSH.
#[derive(Builder, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[builder(builder_type = Builder)]
pub struct Config {
    /// A username.
    #[builder(into)]
    username: Option<String>,

    /// A port.
    #[builder(into)]
    port: usize,
}

impl Config {
    /// Gets the username (if available).
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Gets the port.
    pub fn port(&self) -> usize {
        self.port
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            username: Default::default(),
            port: 22,
        }
    }
}
