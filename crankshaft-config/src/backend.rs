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
    pub name: String,

    /// The type.
    #[serde(flatten)]
    #[builder(into)]
    pub kind: Kind,

    /// The maximum number of concurrent tasks that can run.
    pub max_tasks: usize,

    /// The execution defaults.
    #[builder(into)]
    pub defaults: Option<Defaults>,
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
            .build();

        assert_eq!(config.name, "generic");
        assert!(config.kind.as_generic().is_some());
        assert_eq!(config.max_tasks, 10);

        let defaults = config.defaults.unwrap();
        assert_eq!(defaults.cpu.unwrap(), 1.0);
        assert_eq!(defaults.ram.unwrap(), 16.0);
        assert_eq!(defaults.disk.unwrap(), 250.0);
    }
}
