//! Configuration related to _the command driver_ within a generic execution
//! backend.

pub mod builder;
pub mod locale;
pub mod shell;
pub mod ssh;

pub use builder::Builder;
pub use locale::Locale;
use serde::Deserialize;
use serde::Serialize;
pub use shell::Shell;

/// The default number of times to try the execution of an individual execution
/// within a task.
const DEFAULT_MAX_ATTEMPTS: u32 = 4;

/// A configuration object for a command driver within a generic execution
/// backend.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// The locale within which to run commands.
    locale: Option<Locale>,

    /// The shell to execute within.
    shell: Option<Shell>,

    /// The maximum number of attempts to try a command execution.
    max_attempts: Option<u32>,
}

impl Config {
    /// Creates a new builder for a [`Config`].
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Gets the locale.
    pub fn locale(&self) -> Option<&Locale> {
        self.locale.as_ref()
    }

    /// Gets the shell.
    pub fn shell(&self) -> Option<&Shell> {
        self.shell.as_ref()
    }

    /// Gets the maximum number of attempts.
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts.unwrap_or(DEFAULT_MAX_ATTEMPTS)
    }
}
