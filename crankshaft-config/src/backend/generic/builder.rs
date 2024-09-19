//! Builders for [_generic_ execution backend configuration](Config).

use std::collections::HashMap;

use crate::backend::generic::driver;
use crate::backend::generic::Config;

/// An error related to a [`Builder`].
#[derive(Debug)]
pub enum Error {
    /// A required value was missing for a builder field.
    Missing(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Missing(field) => write!(
                f,
                "missing required value for '{field}' in the generic backend configuration builder"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// A [`Result`](std::result::Result) with an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// A builder for a [generic execution backend configuration object](Config).
#[derive(Default)]
pub struct Builder {
    /// Configuration related to the command driver.
    driver: Option<driver::Config>,

    /// The script used for job submission.
    submit: Option<String>,

    /// A regex used to extract the job id from standard out.
    job_id_regex: Option<String>,

    /// The script used to monitor a submitted job.
    monitor: Option<String>,

    /// The frequency in seconds that the job status will be queried.
    monitor_frequency: Option<u64>,

    /// The script used to kill a job.
    kill: Option<String>,

    /// The runtime attributes.
    attributes: Option<HashMap<String, String>>,
}

impl Builder {
    /// Sets the driver configuration for the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous driver configuration set
    /// within the builder.
    pub fn driver(mut self, config: impl Into<driver::Config>) -> Self {
        self.driver = Some(config.into());
        self
    }

    /// Sets the driver configuration to the default value within the
    /// [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous driver configuration set
    /// within the builder.
    pub fn default_driver(mut self) -> Self {
        self.driver = Some(Default::default());
        self
    }

    /// Sets the submission command for the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous submission commands set
    /// within the builder.
    pub fn submit(mut self, command: impl Into<String>) -> Self {
        self.submit = Some(command.into());
        self
    }

    /// Sets the job id regex for the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous job id regexes set within the
    /// builder.
    pub fn job_id_regex(mut self, regex: impl Into<String>) -> Self {
        self.job_id_regex = Some(regex.into());
        self
    }

    /// Sets the monitor command for the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous monitor commands set within
    /// the builder.
    pub fn monitor(mut self, command: impl Into<String>) -> Self {
        self.monitor = Some(command.into());
        self
    }

    /// Sets the monitor frequency for the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous monitor frequencies set within
    /// the builder.
    pub fn monitor_frequency(mut self, frequency: impl Into<u64>) -> Self {
        self.monitor_frequency = Some(frequency.into());
        self
    }

    /// Sets the kill command for the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous kill commands set within
    /// the builder.
    pub fn kill(mut self, command: impl Into<String>) -> Self {
        self.kill = Some(command.into());
        self
    }

    /// Extends the runtime attributes in the [`Builder`].
    pub fn extend_attrs(mut self, values: impl IntoIterator<Item = (String, String)>) -> Self {
        let mut attributes = self.attributes.unwrap_or_default();
        attributes.extend(values);
        self.attributes = Some(attributes);
        self
    }

    /// Consumes `self` and attempts to build a [`Config`].
    pub fn try_build(self) -> Result<Config> {
        let driver = self.driver.ok_or(Error::Missing("driver"))?;
        let submit = self.submit.ok_or(Error::Missing("submit"))?;
        let monitor = self.monitor.ok_or(Error::Missing("monitor"))?;
        let kill = self.kill.ok_or(Error::Missing("kill"))?;

        Ok(Config {
            driver,
            submit,
            job_id_regex: self.job_id_regex,
            monitor,
            monitor_frequency: self.monitor_frequency,
            kill,
            attributes: self.attributes,
        })
    }
}
