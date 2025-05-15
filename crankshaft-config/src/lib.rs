//! Configuration used within Crankshaft.
//!
//! A few notes on the structure of this crate.
//!
//! * Configuration objects are typically considered immutable and are only able
//!   to be constructed programmatically through the use of one of the builders
//!   (each configuration object should have an associated builder).

use std::collections::HashSet;
use std::path::Path;

use anyhow::bail;
use bon::Builder;
use figment::Figment;
use figment::providers::Format;
use figment::providers::Toml;
use serde::Deserialize;
use serde::Serialize;

pub mod backend;

/// The file name (sans the extension) used when looking for configuration files
/// for Crankshaft.
pub const FILE_NAME: &str = "crankshaft.toml";

/// A global configuration object for Crankshaft.
///
/// When loading, the default sources that are automatically included are:
///
/// * `<CONFIG DIR>/crankshaft/crankshaft.toml`.
/// * `<CWD>/crankshaft.toml`.
/// * If the environment variable is present, the file pointed to by
///   `CRANKSHAFT_CONFIG`.
///
/// Notably, a configuration object may not be valid. You'll need to use the
/// [`validate()`](Config::validate) method to ensure the config is valid.
#[derive(Builder, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[builder(builder_type = Builder)]
pub struct Config {
    /// All registered backends.
    #[builder(into)]
    backends: Vec<backend::Config>,
}

impl Config {
    /// Validates the configuration object.
    pub fn validate(&self) -> anyhow::Result<()> {
        self.backends
            .iter()
            .try_fold(HashSet::new(), |mut found, config| {
                if found.contains(config.name()) {
                    bail!("duplicate backend name: {}", config.name());
                }

                found.insert(config.name());
                Ok(found)
            })?;

        for backend in &self.backends {
            backend.validate()?;
        }

        Ok(())
    }

    /// Gets the configured backends.
    pub fn backends(&self) -> &[backend::Config] {
        self.backends.as_slice()
    }

    /// Consumes `self` and returns the backends.
    pub fn into_backends(self) -> impl Iterator<Item = backend::Config> {
        self.backends.into_iter()
    }

    /// Gets a builder with the default sources preloaded.
    pub fn default_sources() -> Figment {
        let mut builder = Figment::new();

        #[cfg(target_os = "macos")]
        {
            if let Some(home) = dirs::home_dir() {
                builder = builder.admerge(Toml::file(
                    home.join(".config").join("crankshaft").join(FILE_NAME),
                ));
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            if let Some(config_home) = dirs::config_dir() {
                builder =
                    builder.admerge(Toml::file(config_home.join("crankshaft").join(FILE_NAME)));
            }
        }

        if let Ok(mut path) = std::env::current_dir() {
            path.push(FILE_NAME);
            builder = builder.admerge(Toml::file(path));
        }

        if let Ok(config_file) = std::env::var("CRANKSHAFT_CONFIG") {
            builder = builder.admerge(Toml::file(config_file));
        }

        builder
    }

    /// Loads a [`Config`] from the default set of sources.
    ///
    /// The default set of sources are loaded first (see the docs for [`Config`]
    /// for the listed default sources).
    pub fn load() -> figment::Result<Self> {
        Self::default_sources().extract()
    }

    /// Loads the global configuration from a set of sources.
    ///
    /// The default set of sources are loaded first (see the docs for [`Config`]
    /// for the listed default sources). After that, any sources provided in the
    /// `paths` argument is searched.
    pub fn load_with_paths<I, S>(paths: I) -> figment::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<Path>,
    {
        let mut builder = Self::default_sources();

        for path in paths {
            builder = builder.admerge(Toml::file(path.as_ref()));
        }

        builder.extract()
    }

    /// Loads a config from a test fixture.
    #[cfg(test)]
    pub fn fixture(path: impl AsRef<Path>) -> figment::Result<Self> {
        use std::path::PathBuf;

        let mut full_path = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test/fixtures/config/",
        ));

        full_path.push(path);

        Figment::new().admerge(Toml::file(full_path)).extract()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_file_returns_valid_backends() {
        let config = Config::fixture("example.toml").unwrap();
        config.validate().unwrap();
        assert_eq!(config.backends.len(), 3)
    }

    #[test]
    fn loading_config_holds_valid_fields() {
        let config = Config::fixture("example.toml").unwrap();
        config.validate().unwrap();
        let backend = &config.backends[1];

        assert_eq!(backend.name(), "quux");
        assert_eq!(backend.defaults().unwrap().cpu(), Some(1.0));
        assert_eq!(backend.defaults().unwrap().ram(), Some(1.0));
    }

    #[test]
    fn duplicate_names() {
        let config = Config::fixture("duplicate_names.toml").unwrap();
        let err = config.validate().unwrap_err();
        assert_eq!(err.to_string(), "duplicate backend name: test");
    }
}
