//! Builders for the [_Docker_ execution backend configuration](Config).

use crate::backend::docker::Config;
use crate::backend::docker::DEFAULT_CLEANUP;

/// A builder for a [Docker execution backend configuration object](Config).
// **NOTE:** all default values for this struct need to be tested below to
// ensure the defaults never change.
pub struct Builder {
    /// Whether or not to remove the containers after completion of the tasks
    /// (regardless of whether the job was a success or failure).
    cleanup: bool,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            // By default, Docker should clean up containers.
            cleanup: DEFAULT_CLEANUP,
        }
    }
}

impl Builder {
    /// Sets the cleanup property for the [`Builder`].
    ///
    /// # Notes
    ///
    /// This will silently overwrite any previous cleanup properties set within
    /// the builder.
    pub fn cleanup(mut self, cleanup: bool) -> Self {
        self.cleanup = cleanup;
        self
    }

    /// Consumes `self` and returns a built [`Config`].
    pub fn build(self) -> Config {
        Config {
            cleanup: self.cleanup,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let options = Config::default();

        // Docker should clean up containers by default.
        assert!(options.cleanup());
    }
}
