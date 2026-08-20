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
    #[builder(into, default)]
    http: http::Config,

    /// The poll interval, in seconds, to use for querying TES task status.
    interval: Option<u64>,

    /// Whether to read task resource usage from the TES server's task log
    /// metadata.
    ///
    /// The TES specification does not standardize resource usage reporting,
    /// but designates `TaskLog.metadata` for implementation-specific data.
    /// When enabled, tasks are polled with the `BASIC` view (rather than
    /// `MINIMAL`, which omits logs) and the following metadata keys, when
    /// present, are reported as task resource usage events:
    ///
    /// * `peak_rss_bytes` — maximum resident memory, in bytes
    /// * `avg_rss_bytes` — average resident memory, in bytes
    /// * `cpu_time_ms` — total CPU time, in milliseconds
    /// * `user_cpu_time_ms` — user-mode CPU time, in milliseconds
    /// * `system_cpu_time_ms` — system-mode CPU time, in milliseconds
    /// * `disk_used_bytes` — disk space used, in bytes
    ///
    /// Values may be JSON numbers or numeric strings. Servers that do not
    /// populate these keys simply produce no resource usage events.
    ///
    /// Defaults to `false`.
    #[serde(default)]
    #[builder(default)]
    resource_usage_metadata: bool,
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

    /// Gets the poll interval, in seconds, for querying TES task status.
    pub fn interval(&self) -> Option<u64> {
        self.interval
    }

    /// Gets whether task resource usage is read from the TES server's task
    /// log metadata.
    pub fn resource_usage_metadata(&self) -> bool {
        self.resource_usage_metadata
    }

    /// Consumes `self` and returns the constituent, owned parts of the
    /// configuration.
    pub fn into_parts(self) -> (Url, http::Config, Option<u64>, bool) {
        (
            self.url,
            self.http,
            self.interval,
            self.resource_usage_metadata,
        )
    }
}
