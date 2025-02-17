//! Configuration related to the locale from within which commands are executed.

use serde::Deserialize;
use serde::Serialize;

use crate::backend::generic::driver::ssh;

/// The environment from which jobs are executed.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "PascalCase")]
pub enum Locale {
    /// Local execution.
    #[default]
    Local,

    /// Remote execution over SSH.
    SSH {
        /// The host for the connection.
        host: String,

        /// Any options for the SSH connection.
        #[serde(default)]
        options: ssh::Config,
    },
}
