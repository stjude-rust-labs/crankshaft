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

/// The maximum number of attempts for a driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct MaxAttempts(u32);

impl MaxAttempts {
    /// Gets a copy of the inner value.
    pub fn inner(&self) -> u32 {
        self.0
    }
}

impl From<u32> for MaxAttempts {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl Default for MaxAttempts {
    fn default() -> Self {
        Self(DEFAULT_MAX_ATTEMPTS)
    }
}

/// A configuration object for a command driver within a generic execution
/// backend.
#[derive(Builder, Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[builder(builder_type = Builder)]
pub struct Config {
    /// The locale within which to run commands.
    #[builder(into)]
    pub locale: Option<Locale>,

    /// The shell to execute within.
    #[builder(into)]
    pub shell: Option<Shell>,

    /// The maximum number of attempts to try a command execution.
    pub max_attempts: Option<MaxAttempts>,
}

/// A driver configuration used during testing.
#[cfg(test)]
pub(crate) fn demo() -> Config {
    Config::builder()
        .locale(Locale::Local)
        .shell(Shell::Bash)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo() {
        let demo = super::demo();
        assert_eq!(demo.locale.unwrap(), Locale::Local);
        assert_eq!(demo.shell.unwrap(), Shell::Bash);
        assert_eq!(demo.max_attempts.unwrap_or_default().inner(), 4);
    }
}
