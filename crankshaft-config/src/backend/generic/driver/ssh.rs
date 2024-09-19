//! Configuration related to an SSH-based command drivers.

use serde::Deserialize;
use serde::Serialize;

/// A builder for [`Config`].
pub struct Builder(Config);

impl Builder {
    /// Adds a username to the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous username declarations
    /// provided to the builder.
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.0.username = Some(username.into());
        self
    }

    /// Consumes `self` and returns a built [`Config`].
    pub fn build(self) -> Config {
        self.0
    }
}

/// Configuration related to SSH.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// A username.
    pub username: Option<String>,

    /// A port.
    pub port: usize,
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
