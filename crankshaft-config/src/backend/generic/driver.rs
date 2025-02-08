//! Configuration related to _the command driver_ within a generic execution
//! backend.

pub mod locale;
pub mod shell;
pub mod ssh;

use bon::Builder;
pub use locale::Locale;
use serde::Deserialize;
use serde::Serialize;
pub use shell::Shell;

/// The default number of times to try the execution of an individual execution
/// within a task.
const DEFAULT_MAX_ATTEMPTS: u32 = 4;

/// A configuration object for a command driver within a generic execution
/// backend.
#[derive(Builder, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[builder(builder_type = Builder)]
pub struct Config {
    /// The locale within which to run commands.
    #[builder(into)]
    locale: Option<Locale>,

    /// The shell to execute within.
    #[builder(into)]
    shell: Option<Shell>,

    /// The maximum number of attempts to try a command execution.
    max_attempts: Option<u32>,
}

impl Config {
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
