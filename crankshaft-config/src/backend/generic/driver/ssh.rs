//! Configuration related to an SSH-based command drivers.

use bon::Builder;
use serde::Deserialize;
use serde::Serialize;

/// Configuration related to SSH.
#[derive(Builder, Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[builder(builder_type = Builder)]
pub struct Config {
    /// The host for the connection.
    #[builder(into)]
    host: String,

    /// The port for the connection.
    #[builder(default = 22)]
    port: u16,

    /// The SSH username.
    #[builder(into)]
    username: Option<String>,
}

impl Config {
    /// Gets the SSH host.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Gets the SSH port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Gets the username (if available).
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Converts the configuration into its parts.
    pub fn into_parts(self) -> (String, u16, Option<String>) {
        (self.host, self.port, self.username)
    }
}
