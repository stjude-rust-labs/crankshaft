//! Configuration related to _generic_ execution backends.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::LazyLock;

use bon::Builder;
use bon::builder;
use regex::Captures;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

pub mod driver;

/// An error related to unexpected remaining substitution tokens in a (otherwise
/// presumed to be fully resolved) command.
#[derive(Error, Debug)]
#[error("unresolved substitutions in command `{command}`")]
pub struct UnresolvedSubstitutionError {
    /// The command containing the unresolved substitutions.
    command: String,
}

/// A result from substitutions that might contain a
/// [`UnresolvedSubstitutionError`].
pub type ResolveResult = std::result::Result<String, UnresolvedSubstitutionError>;

/// The regex to use when replacing whitespace.
static WHITESPACE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // SAFETY: this will always trivially unwrap.
    Regex::new(r"\s+").unwrap()
});

/// The regex to use when replacing keys in generic backend values.
static PLACEHOLDER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // SAFETY: this is checked statically to ensure it always unwraps.
    Regex::new(r"~\{([^}]*)\}").unwrap()
});

/// Replaces placeholders within a generic configuration value.
pub fn substitute(input: &str, replacements: &HashMap<Cow<'_, str>, Cow<'_, str>>) -> String {
    PLACEHOLDER_REGEX
        .replace_all(input, |captures: &Captures<'_>| {
            // SAFETY: the `PLACEHOLDER_REGEX` above is hardcoded to ensure a group
            // is included. This is tested statically below.
            let key = &captures.get(1).unwrap();

            replacements
                .get(key.as_str())
                .map(|r| r.as_ref().to_string())
                .unwrap_or_else(|| format!("~{{{key}}}", key = key.as_str()))
        })
        .to_string()
}

/// A configuration object for a generic execution backend.
#[derive(Builder, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[builder(builder_type = Builder)]
pub struct Config {
    /// Configuration related to the command driver.
    #[serde(flatten)]
    #[builder(into)]
    driver: driver::Config,

    /// The script used for job submission.
    #[builder(into)]
    submit: String,

    /// A regex used to extract the job id from standard out.
    #[builder(into)]
    job_id_regex: Option<String>,

    /// The script used to monitor a submitted job.
    #[builder(into)]
    monitor: String,

    /// The frequency in seconds that the job status will be queried.
    #[builder(into)]
    monitor_frequency: Option<u64>,

    /// The script used to kill a job.
    #[builder(into)]
    kill: String,

    /// The runtime attributes.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[builder(into, default)]
    attributes: HashMap<Cow<'static, str>, Cow<'static, str>>,
}

impl Config {
    /// A utility method to perform the substitutions on a particular command at
    /// runtime.
    ///
    /// Here, `command` means the the command specified in the configuration
    /// file (known statically) and `exec` means the args to be executed that
    /// were received from the task.
    ///
    /// **NOTE:** that the only reason this method really exists rather than
    /// performing the substitution down in the methods themselves is because
    /// it's rather ugly for performance reasons: to avoid cloning the
    /// entire runtime attributes HashMap each time a substitution was
    /// performed, we first do substitution of the script followed by
    /// substitution of the runtime attributes.
    #[inline]
    fn resolve(
        &self,
        command: &str,
        substitutions: &HashMap<Cow<'_, str>, Cow<'_, str>>,
    ) -> ResolveResult {
        let mut result = substitute(command, substitutions);

        if !self.attributes.is_empty() {
            result = substitute(&result, &self.attributes);
        }

        // NOTE: this is just to help clean up some of the output. The intention
        // is to remove line breaks and multiple spaces that make it easier to
        // format the command in configs. I recognize that it incurs another
        // allocation, but it seemed worth it overall for readability.
        let result = WHITESPACE_REGEX.replace_all(result.trim(), " ").to_string();

        if PLACEHOLDER_REGEX.is_match(&result) {
            Err(UnresolvedSubstitutionError { command: result })
        } else {
            Ok(result)
        }
    }

    /// Gets the driver configuration.
    pub fn driver(&self) -> &driver::Config {
        &self.driver
    }

    /// Gets the submit command.
    pub fn submit(&self) -> &str {
        &self.submit
    }

    /// Gets the job id regex.
    pub fn job_id_regex(&self) -> Option<&str> {
        self.job_id_regex.as_deref()
    }

    /// Gets the monitor command.
    pub fn monitor(&self) -> &str {
        self.monitor.as_ref()
    }

    /// Gets the monitor frequency (in seconds).
    pub fn monitor_frequency(&self) -> Option<u64> {
        self.monitor_frequency
    }

    /// Gets the kill command.
    pub fn kill(&self) -> &str {
        self.kill.as_ref()
    }

    /// Gets the runtime attributes.
    pub fn attributes(&self) -> &HashMap<Cow<'static, str>, Cow<'static, str>> {
        &self.attributes
    }

    /// Gets the submit command with all of the substitutions resolved.
    pub fn resolve_submit(
        &self,
        substitutions: &HashMap<Cow<'_, str>, Cow<'_, str>>,
    ) -> ResolveResult {
        self.resolve(&self.submit, substitutions)
    }

    /// Gets the monitor command with all of the substitutions resolved.
    pub fn resolve_monitor(
        &self,
        substitutions: &HashMap<Cow<'_, str>, Cow<'_, str>>,
    ) -> ResolveResult {
        self.resolve(&self.monitor, substitutions)
    }

    /// Gets the kill command with all of the substitutions resolved.
    pub fn resolve_kill(
        &self,
        substitutions: &HashMap<Cow<'_, str>, Cow<'_, str>>,
    ) -> ResolveResult {
        self.resolve(&self.kill, substitutions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_placeholder_regex_unwraps() {
        let _ = PLACEHOLDER_REGEX;
    }

    #[test]
    fn replacement_works() -> Result<(), Box<dyn std::error::Error>> {
        let mut replacements = HashMap::new();
        replacements.insert("foo".into(), "bar".into());

        assert_eq!(substitute("hello, ~{foo}", &replacements), "hello, bar");

        Ok(())
    }
}
