//! Configuration related to the _Docker_ execution backend.

mod builder;

pub use builder::Builder;
use serde::Deserialize;
use serde::Serialize;

/// The default value for cleaning up Docker containers.
pub const DEFAULT_CLEANUP: bool = true;

/// A utility function used to set the default value for `cleanup` via serde.
fn default_cleanup() -> bool {
    DEFAULT_CLEANUP
}

/// A configuration object for a Docker execution backend.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Whether or not to remove the containers after completion of the tasks
    /// (regardless of whether the job was a success or failure).
    #[serde(default = "default_cleanup")]
    cleanup: bool,
}

impl Config {
    /// Gets a builder for [`Config`].
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Gets whether the backend is configured to remove the containers after
    /// completion of the tasks (regardless of whether the job was a success or
    /// failure).
    pub fn cleanup(&self) -> bool {
        self.cleanup
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_unwraps() {
        Config::default();
    }
}
