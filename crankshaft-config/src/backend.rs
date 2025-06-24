//! Configuration related to execution backends.

use bon::Builder;
use serde::Deserialize;
use serde::Serialize;

mod defaults;
pub mod docker;
pub mod generic;
mod kind;
pub mod tes;

pub use defaults::Defaults;
pub use kind::Kind;

/// A configuration object for an execution backend.
#[derive(Builder, Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
#[builder(builder_type = Builder)]
pub struct Config {
    /// The name.
    #[builder(into)]
    name: String,

    /// The type.
    #[serde(flatten)]
    #[builder(into)]
    kind: Kind,

    /// The maximum number of concurrent tasks that can run.
    max_tasks: usize,

    /// The execution defaults.
    #[builder(into)]
    defaults: Option<Defaults>,

    /// The monitoring flag
    monitoring: bool,
}

impl Config {
    /// Gets the name of the backend.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the kind of the backend.
    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    /// Gets the maximum number of tasks.
    pub fn max_tasks(&self) -> usize {
        self.max_tasks
    }

    /// Gets the execution defaults of the backend.
    pub fn defaults(&self) -> Option<&Defaults> {
        self.defaults.as_ref()
    }

    /// Consumes `self` returns the constituent, owned parts of the
    /// configuration.
    pub fn into_parts(self) -> (String, Kind, usize, Option<Defaults>, bool) {
        (
            self.name,
            self.kind,
            self.max_tasks,
            self.defaults,
            self.monitoring,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic() {
        let config = Config::builder()
            .name("generic")
            .kind(Kind::Generic(generic::demo()))
            .max_tasks(10)
            .defaults(Defaults::builder().cpu(1.0).ram(16.0).disk(250.0).build())
            .monitoring(false)
            .build();

        assert_eq!(config.name(), "generic");
        assert!(config.kind().as_generic().is_some());
        assert_eq!(config.max_tasks(), 10);

        let defaults = config.defaults.unwrap();
        assert_eq!(defaults.cpu().unwrap(), 1.0);
        assert_eq!(defaults.ram().unwrap(), 16.0);
        assert_eq!(defaults.disk().unwrap(), 250.0);
    }
}
