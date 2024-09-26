//! Builders for [command driver configuration](Config).

use crate::backend::generic::driver::Config;
use crate::backend::generic::driver::DEFAULT_MAX_ATTEMPTS;
use crate::backend::generic::driver::Locale;
use crate::backend::generic::driver::Shell;
use crate::backend::generic::driver::ssh;

/// A builder for a [command driver configuration object](Config).
pub struct Builder {
    /// The locale, if it has been set.
    locale: Option<Locale>,

    /// The shell, if it has been set.
    shell: Option<Shell>,

    /// The maximum number of attempts to try a command execution.
    max_attempts: Option<u32>,
}

impl Builder {
    /// Creates a new [`Builder`] with no constituent parts set.
    pub fn empty() -> Self {
        Default::default()
    }

    /// Configures the generic backend to execute commands on the local machine.
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous locale declarations provided
    /// to the builder.
    pub fn with_local(mut self) -> Self {
        self.locale = Some(Locale::Local);
        self
    }

    /// Configures the generic backend to localize to a remote machine over SSH
    /// before executing commands.
    ///
    /// Note that this is related to the host that _submits_ the commands—not
    /// necessarily the one that executes them. So, for example, if you wanted
    /// to connect to a head node within an HPC before running job submission
    /// commands, you'd want to use this option.
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous locale declarations provided
    /// to the builder.
    pub fn with_ssh(mut self, host: impl Into<String>, options: ssh::Config) -> Self {
        self.locale = Some(Locale::SSH {
            host: host.into(),
            options,
        });

        self
    }

    /// Configures the generic backend to use the specified shell for execution.
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous shell declarations provided to
    /// the builder.
    pub fn shell(mut self, shell: Shell) -> Self {
        self.shell = Some(shell);
        self
    }

    /// Configures the generic backend to use a maximum number of attempts when
    /// submitting jobs.
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous maximum attempts declarations
    /// provided to the builder.
    pub fn max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = Some(max_attempts);
        self
    }

    /// Configures the generic backend to execute commands on the local machine.
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous locale declarations provided
    /// to the builder.
    pub fn locale(mut self) -> Self {
        self.locale = Some(Locale::Local);
        self
    }

    /// Consumes `self` and builds a [`Config`].
    pub fn build(self) -> Config {
        Config {
            locale: self.locale,
            shell: self.shell,
            max_attempts: self.max_attempts,
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            locale: Some(Locale::default()),
            shell: Some(Shell::default()),
            max_attempts: Some(DEFAULT_MAX_ATTEMPTS),
        }
    }
}
