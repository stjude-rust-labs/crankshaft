//! Configuration related to the _Docker_ execution backend.

use bon::Builder;
use serde::Deserialize;
use serde::Serialize;

/// The default value for cleaning up Docker containers.
pub const DEFAULT_CLEANUP: bool = true;

/// A utility function used to set the default value for `cleanup` via serde.
fn default_cleanup() -> bool {
    DEFAULT_CLEANUP
}

/// Configuration for events emitted by the Docker execution backend.
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EventConfig {
    /// Whether or not to send the task stdout event.
    pub send_stdout: bool,
    /// Whether or not to send the task stderr event.
    pub send_stderr: bool,
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            send_stdout: true,
            send_stderr: true,
        }
    }
}

/// A configuration object for a Docker execution backend.
#[derive(Builder, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[builder(builder_type = Builder)]
pub struct Config {
    /// Whether or not to remove the containers after completion of the tasks
    /// (regardless of whether the job was a success or failure).
    #[serde(default = "default_cleanup")]
    #[builder(default = DEFAULT_CLEANUP)]
    cleanup: bool,
    /// Configuration for events emitted by the Docker execution backend.
    #[serde(default)]
    #[builder(default)]
    events: EventConfig,
    /// The interval, in seconds, at which to sample a running container's
    /// resource usage and emit task resource usage events.
    ///
    /// When unset, resource usage is not sampled and no resource usage events
    /// are emitted.
    #[serde(default)]
    #[builder(into)]
    resource_usage_interval: Option<u64>,
}

impl Config {
    /// Gets whether the backend is configured to remove the containers after
    /// completion of the tasks (regardless of whether the job was a success or
    /// failure).
    pub fn cleanup(&self) -> bool {
        self.cleanup
    }

    /// Gets the interval, in seconds, at which to sample a running
    /// container's resource usage.
    ///
    /// Returns `None` when resource usage sampling is disabled.
    pub fn resource_usage_interval(&self) -> Option<u64> {
        self.resource_usage_interval
    }

    /// Gets the event configuration for the backend.
    pub fn events(&self) -> EventConfig {
        self.events
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
