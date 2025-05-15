//! Kinds of execution backends.

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::backend::docker;
use crate::backend::generic;
use crate::backend::tes;

/// A kind of execution backend.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "PascalCase")]
pub enum Kind {
    /// A Docker backend.
    Docker(docker::Config),

    /// A generic backend.
    Generic(generic::Config),

    /// A TES backend.
    TES(tes::Config),
}

impl Kind {
    /// Validates the backend kind configuration object.
    pub fn validate(&self) -> Result<()> {
        match self {
            Kind::Docker(config) => config.validate(),
            Kind::Generic(config) => config.validate(),
            Kind::TES(config) => config.validate(),
        }
    }

    /// Attempts to return a reference to the inner [docker
    /// configuration][`docker::Config`].
    pub fn as_docker(&self) -> Option<&docker::Config> {
        match self {
            Kind::Docker(config) => Some(config),
            _ => None,
        }
    }

    /// Consumes `self` and attempts to return an inner [docker
    /// configuration][`docker::Config`].
    pub fn into_docker(self) -> Option<docker::Config> {
        match self {
            Kind::Docker(config) => Some(config),
            _ => None,
        }
    }

    /// Consumes `self` and returns an inner [docker
    /// configuration][`docker::Config`].
    ///
    /// # Panics
    ///
    /// If the inner kind is not [`Kind::Docker`].
    pub fn unwrap_docker(self) -> docker::Config {
        match self {
            Kind::Docker(config) => config,
            _ => panic!("the inner kind is not `Kind::Docker`"),
        }
    }

    /// Attempts to return a reference to the inner [generic
    /// configuration][`generic::Config`].
    pub fn as_generic(&self) -> Option<&generic::Config> {
        match self {
            Kind::Generic(config) => Some(config),
            _ => None,
        }
    }

    /// Consumes `self` and attempts to return an inner [generic
    /// configuration][`generic::Config`].
    pub fn into_generic(self) -> Option<generic::Config> {
        match self {
            Kind::Generic(config) => Some(config),
            _ => None,
        }
    }

    /// Consumes `self` and returns an inner [generic
    /// configuration][`generic::Config`].
    ///
    /// # Panics
    ///
    /// If the inner kind is not [`Kind::Generic`].
    pub fn unwrap_generic(self) -> generic::Config {
        match self {
            Kind::Generic(config) => config,
            _ => panic!("the inner kind is not `Kind::Generic`"),
        }
    }

    /// Attempts to return a reference to the inner [TES
    /// configuration][`tes::Config`].
    pub fn as_tes(&self) -> Option<&tes::Config> {
        match self {
            Kind::TES(config) => Some(config),
            _ => None,
        }
    }

    /// Consumes `self` and attempts to return an inner [TES
    /// configuration][`tes::Config`].
    pub fn into_tes(self) -> Option<tes::Config> {
        match self {
            Kind::TES(config) => Some(config),
            _ => None,
        }
    }

    /// Consumes `self` and returns an inner [TES configuration][`tes::Config`].
    ///
    /// # Panics
    ///
    /// If the inner kind is not [`Kind::TES`].
    pub fn unwrap_tes(self) -> tes::Config {
        match self {
            Kind::TES(config) => config,
            _ => panic!("the inner kind is not `Kind::TES`"),
        }
    }
}
