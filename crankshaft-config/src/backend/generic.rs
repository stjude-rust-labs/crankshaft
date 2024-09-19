//! Configuration related to _generic_ execution backends.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Captures;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;

mod builder;
pub mod driver;

pub use builder::Builder;

/// An error related to unexpected remaining substitution tokens in a (otherwise
/// presumed to be fully resolved) command.
#[derive(Debug)]
pub struct UnresolvedSubstitutionError {
    /// The command containing the unresolved substitutions.
    command: String,
}

impl std::fmt::Display for UnresolvedSubstitutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unresolved substitutions in command: {}", self.command)
    }
}

impl std::error::Error for UnresolvedSubstitutionError {}

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
pub fn substitute(input: &str, replacements: &HashMap<String, String>) -> String {
    PLACEHOLDER_REGEX
        .replace_all(input, |captures: &Captures<'_>| {
            // SAFETY: the `PLACEHOLDER_REGEX` above is hardcoded to ensure a group
            // is included. This is tested statically below.
            let key = &captures.get(1).unwrap();

            replacements
                .get(key.as_str())
                .unwrap_or(&format!("~{{{}}}", key.as_str()))
                .to_string()
        })
        .to_string()
}

/// A configuration object for a generic execution backend.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Configuration related to the command driver.
    #[serde(flatten)]
    driver: driver::Config,

    /// The script used for job submission.
    submit: String,

    /// A regex used to extract the job id from standard out.
    job_id_regex: Option<String>,

    /// The script used to monitor a submitted job.
    monitor: String,

    /// The frequency in seconds that the job status will be queried.
    monitor_frequency: Option<u64>,

    /// The script used to kill a job.
    kill: String,

    /// The runtime attributes.
    attributes: Option<HashMap<String, String>>,
}

impl Config {
    /// Gets a default [`Builder`] for a [`Config`].
    pub fn builder() -> Builder {
        Builder::default()
    }

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
    // TODO(clay): could this be used with `Cow<'a, str>`?
    #[inline]
    fn resolve(&self, command: &str, substitutions: &HashMap<String, String>) -> ResolveResult {
        let mut result = substitute(command, substitutions);

        if let Some(attrs) = self.attributes() {
            result = substitute(&result, attrs);
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
    pub fn attributes(&self) -> Option<&HashMap<String, String>> {
        self.attributes.as_ref()
    }

    /// Gets the submit command with all of the substitutions resolved.
    pub fn resolve_submit(&self, substitutions: &HashMap<String, String>) -> ResolveResult {
        self.resolve(&self.submit, substitutions)
    }

    /// Gets the monitor command with all of the substitutions resolved.
    pub fn resolve_monitor(&self, substitutions: &HashMap<String, String>) -> ResolveResult {
        self.resolve(&self.monitor, substitutions)
    }

    /// Gets the kill command with all of the substitutions resolved.
    pub fn resolve_kill(&self, substitutions: HashMap<String, String>) -> ResolveResult {
        self.resolve(&self.kill, &substitutions)
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
        replacements.insert(String::from("foo"), String::from("bar"));

        assert_eq!(substitute("hello, ~{foo}", &replacements), "hello, bar");

        Ok(())
    }
}
