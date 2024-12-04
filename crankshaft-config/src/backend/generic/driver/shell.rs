//! Configuration related to commands executed within a shell.

use std::ffi::OsString;

use serde::Deserialize;
use serde::Serialize;

/// The expected path for the `env` binary.
const ENV_PATH: &str = "/usr/bin/env";

/// A shell within which to run commands.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Shell {
    /// Run commands using `bash`.
    #[default]
    Bash,

    /// Run commands using `sh`.
    Sh,
}

impl Shell {
    /// Gets a series of args that can be passed through to a driver for
    /// commands.
    pub fn args<I, S>(&self, args: I) -> impl Iterator<Item = OsString> + use<I, S>
    where
        I: IntoIterator<Item = OsString>,
    {
        let base_args = match self {
            Shell::Bash => [
                OsString::from(ENV_PATH),
                OsString::from("bash"),
                OsString::from("-c"),
            ],
            Shell::Sh => [
                OsString::from(ENV_PATH),
                OsString::from("sh"),
                OsString::from("-c"),
            ],
        };

        base_args.into_iter().chain(args)
    }
}
