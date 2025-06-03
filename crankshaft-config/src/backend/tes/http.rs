//! Configuration related to HTTP within the TES execution backend.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use serde::Serialize;

/// Represents HTTP authentication configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum HttpAuthConfig {
    /// Use basic authentication.
    Basic {
        /// The username for the authentication.
        username: String,
        /// The password for the authentication.
        password: String,
    },
    /// Use bearer token authentication.
    Bearer {
        /// The bearer token for authentication.
        token: String,
    },
}

impl HttpAuthConfig {
    /// Gets the `Authorization` header value based on the config.
    pub fn header_value(&self) -> String {
        match self {
            Self::Basic { username, password } => format!(
                "Basic {encoded}",
                encoded = STANDARD.encode(format!("{username}:{password}"))
            ),
            Self::Bearer { token } => format!("Bearer {token}"),
        }
    }
}

/// A configuration object for HTTP settings within the TES execution backend.
// **NOTE:** all default values for this struct need to be tested below to
// ensure the defaults never change.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// The HTTP authentication to use.
    pub auth: Option<HttpAuthConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let options = Config::default();
        assert!(options.auth.is_none());
    }
}
