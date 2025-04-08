//! Configuration used within Crankshaft.
//!
//! A few notes on the structure of this crate.
//!
//! * Configuration objects are typically considered immutable and are only able
//!   to be constructed programmatically through the use of one of the builders
//!   (each configuration object should have an associated builder).

use std::path::Path;

use bon::Builder;
use config::Config as ConfigCrate;
use config::ConfigBuilder;
use config::ConfigError as Error;
use config::Environment;
use config::File;
use config::builder::DefaultState;
use serde::Deserialize;
use serde::Serialize;

pub mod backend;

/// The prefix for any environment variables that influence the configuration of
/// Crankshaft.
pub const ENV_PREFIX: &str = "CRANKSHAFT";

/// The file name (sans the extension) used when looking for configuration files
/// for Crankshaft.
///
/// E.g., if this value is `"Crankshaft"`, then `Crankshaft.toml`,
/// `Crankshaft.json`, and `Crankshaft.yaml` will all be recognized.
pub const FILE_NAME: &str = "Crankshaft";

/// A [`Result`](std::result::Result) with an [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// A global configuration object for Crankshaft.
///
/// When loading, the default sources that are automatically included are:
///
/// * `<CONFIG DIR>/crankshaft/Crankshaft.toml`.
/// * `<CWD>/Crankshaft.toml`.
/// * Environment variables starting with `CRANKSHAFT_`.
#[derive(Builder, Deserialize, Serialize, Debug)]
#[serde(rename_all = "kebab-case")]
#[builder(builder_type = Builder)]
pub struct Config {
    /// All registered backends.
    #[builder(into)]
    pub backends: Vec<backend::Config>,
}

impl Config {
    /// Gets a builder with the default sources preloaded.
    fn default_sources() -> ConfigBuilder<DefaultState> {
        let mut builder = ConfigCrate::builder();

        if let Some(mut path) = dirs::config_dir() {
            path.push("crankshaft");
            path.push(FILE_NAME);
            builder = builder.add_source(File::from(path));
        }

        if let Ok(mut path) = std::env::current_dir() {
            path.push(FILE_NAME);
            builder = builder.add_source(File::from(path));
        }

        builder.add_source(Environment::with_prefix(ENV_PREFIX))
    }

    /// Loads a [`Config`] from the default set of sources.
    ///
    /// The default set of sources are loaded first (see the docs for [`Config`]
    /// for the listed default sources).
    pub fn load() -> Result<Self> {
        Self::default_sources().build()?.try_deserialize()
    }

    /// Loads the global configuration from a set of sources.
    ///
    /// The default set of sources are loaded first (see the docs for [`Config`]
    /// for the listed default sources). After that, any sources provided in the
    /// `paths` argument is searched.
    pub fn load_with_paths<I, S>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<Path>,
    {
        let mut builder = Self::default_sources();

        for path in paths {
            builder = builder.add_source(File::from(path.as_ref()));
        }

        builder.build()?.try_deserialize()
    }

    /// Loads a config from a test fixture.
    #[cfg(test)]
    pub fn fixture(path: impl AsRef<Path>) -> Result<Self> {
        use std::path::PathBuf;

        let mut full_path = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test/fixtures/config/",
        ));

        full_path.push(path);

        ConfigCrate::builder()
            .add_source(File::from(full_path))
            .build()?
            .try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn loading_file_returns_valid_backends() {
        let config = Config::fixture("example.toml").unwrap();
        assert_eq!(config.backends.len(), 3)
    }

    #[test]
    fn loading_config_holds_valid_fields() {
        let config = Config::fixture("example.toml").unwrap();
        let backend = &config.backends[1];

        assert_eq!(backend.name, "quux");
        assert_eq!(backend.defaults.as_ref().unwrap().cpu, Some(1.0));
        assert_eq!(backend.defaults.as_ref().unwrap().ram, Some(1.0));
    }
}
