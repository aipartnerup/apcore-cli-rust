// apcore-cli — Integration tests for AuthProvider.
// Protocol spec: SEC-02

use apcore_cli::config::ConfigResolver;
use apcore_cli::security::auth::{AuthProvider, AuthenticationError};

#[test]
fn test_get_api_key_from_env_var() {
    // APCORE_API_KEY must be returned when set.
    // TODO: set env var, create AuthProvider, call get_api_key(), assert key.
    assert!(false, "not implemented");
}

#[test]
fn test_get_api_key_missing_returns_error() {
    // When no key is available, MissingApiKey must be returned.
    // TODO: clear all key sources, assert MissingApiKey.
    assert!(false, "not implemented");
}

#[test]
fn test_authenticate_request_adds_bearer_header() {
    // authenticate_request must add Authorization: Bearer <key> header.
    // TODO: mock reqwest builder, verify header is added.
    assert!(false, "not implemented");
}

#[test]
fn test_handle_response_ok_returns_response() {
    // A 200 response must be returned unchanged.
    assert!(false, "not implemented");
}

#[test]
fn test_handle_response_401_returns_invalid_key_error() {
    // A 401 response must yield AuthenticationError::InvalidApiKey.
    assert!(false, "not implemented");
}
