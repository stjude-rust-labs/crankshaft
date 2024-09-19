//! Configuration related to execution backends.

use serde::Deserialize;
use serde::Serialize;

mod builder;
mod defaults;
pub mod docker;
pub mod generic;
mod kind;
pub mod tes;

pub use builder::Builder;
pub use defaults::Defaults;
pub use kind::Kind;

/// A configuration object for an execution backend.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// The name.
    name: String,

    /// The type.
    #[serde(flatten)]
    kind: Kind,

    /// The maximum number of concurrent tasks that can run.
    max_tasks: usize,

    /// The execution defaults.
    defaults: Option<Defaults>,
}

impl Config {
    /// Gets a default [`Builder`] for a [`Config`].
    pub fn builder() -> Builder {
        Builder::default()
    }

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

    /// Consumes `self` returns the constituent parts of the [`Config`].
    pub fn into_parts(self) -> (String, Kind, usize, Option<Defaults>) {
        (self.name, self.kind, self.max_tasks, self.defaults)
    }
}
