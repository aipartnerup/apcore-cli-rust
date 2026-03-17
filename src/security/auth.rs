// apcore-cli — Authentication provider.
// Protocol spec: SEC-02 (AuthProvider, AuthenticationError)

use thiserror::Error;

use crate::config::ConfigResolver;

// ---------------------------------------------------------------------------
// AuthenticationError
// ---------------------------------------------------------------------------

/// Errors produced by authentication operations.
#[derive(Debug, Error)]
pub enum AuthenticationError {
    /// No API key is configured or stored in the keyring.
    #[error("no API key found; run `apcore-cli auth login` to configure")]
    MissingApiKey,

    /// The stored API key was rejected by the server.
    #[error("authentication failed: invalid or expired API key")]
    InvalidApiKey,

    /// The keyring could not be accessed.
    #[error("keyring error: {0}")]
    KeyringError(String),

    /// Network or HTTP error during authentication check.
    #[error("authentication request failed: {0}")]
    RequestError(String),
}

// ---------------------------------------------------------------------------
// AuthProvider
// ---------------------------------------------------------------------------

/// Provides API key retrieval and HTTP request authentication for the CLI.
///
/// API key resolution order:
/// 1. Environment variable `APCORE_API_KEY`
/// 2. System keyring (service: `"apcore-cli"`, user: current OS user)
/// 3. Config file `auth.api_key` field
pub struct AuthProvider {
    config: ConfigResolver,
}

impl AuthProvider {
    /// Create a new `AuthProvider` with the given configuration resolver.
    pub fn new(config: ConfigResolver) -> Self {
        Self { config }
    }

    /// Retrieve the API key using the resolution order above.
    ///
    /// # Errors
    /// Returns `AuthenticationError::MissingApiKey` when no key is found.
    pub fn get_api_key(&self) -> Result<String, AuthenticationError> {
        // TODO: check env var, then keyring, then config file.
        todo!("AuthProvider::get_api_key")
    }

    /// Inject the Authorization header into the given request builder.
    ///
    /// # Errors
    /// Returns `AuthenticationError` if the key cannot be retrieved.
    pub fn authenticate_request(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, AuthenticationError> {
        // TODO: call get_api_key(), add Bearer token header.
        let _ = builder;
        todo!("AuthProvider::authenticate_request")
    }

    /// Inspect an HTTP response for 401/403 codes and raise the appropriate error.
    ///
    /// Returns the response unchanged if authentication succeeded.
    pub fn handle_response(
        &self,
        response: reqwest::Response,
    ) -> Result<reqwest::Response, AuthenticationError> {
        // TODO: check status code, map 401 → InvalidApiKey, return others as-is.
        let _ = response;
        todo!("AuthProvider::handle_response")
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_api_key_from_env_var() {
        // APCORE_API_KEY env var must be returned by get_api_key().
        assert!(false, "not implemented");
    }

    #[test]
    fn test_get_api_key_missing_returns_error() {
        // When no key is available, MissingApiKey must be returned.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_authenticate_request_adds_bearer_header() {
        // authenticate_request must add an Authorization: Bearer … header.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_handle_response_401_returns_error() {
        // A 401 response must yield AuthenticationError::InvalidApiKey.
        assert!(false, "not implemented");
    }
}
