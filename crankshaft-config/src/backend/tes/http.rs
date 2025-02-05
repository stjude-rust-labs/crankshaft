//! Configuration related to HTTP within the TES execution backend.

use serde::Deserialize;
use serde::Serialize;

/// A configuration object for HTTP settings within the TES execution backend.
// **NOTE:** all default values for this struct need to be tested below to
// ensure the defaults never change.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// If needed, the basic auth token to provide to the service.
    pub basic_auth_token: Option<String>,
}

impl Config {
    /// Gets the basic auth token (if it exists).
    pub fn basic_auth_token(&self) -> Option<&str> {
        self.basic_auth_token.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let options = Config::default();
        assert_eq!(options.basic_auth_token, None);
    }
}
