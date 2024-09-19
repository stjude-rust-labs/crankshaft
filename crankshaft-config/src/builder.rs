//! Builders for a [global configuration object for Crankshaft](Config).

use crate::backend;
use crate::Config;

/// A builder for a [global configuration object for Crankshaft](Config).
#[derive(Default)]
pub struct Builder {
    /// All registered backends.
    backends: Vec<backend::Config>,
}

impl Builder {
    /// Adds a backend to the [`Builder`].
    pub fn push_backend(mut self, config: backend::Config) -> Self {
        self.backends.push(config);
        self
    }

    /// Consumes `self` and builds a [`Config`].
    pub fn build(self) -> Config {
        Config {
            backends: self.backends,
        }
    }
}
