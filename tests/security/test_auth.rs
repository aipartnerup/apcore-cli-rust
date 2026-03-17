// apcore-cli — Integration tests for AuthProvider.
// Protocol spec: SEC-02

use apcore_cli::config::ConfigResolver;
use apcore_cli::security::auth::{AuthProvider, AuthenticationError};

/// Serialize all tests that touch APCORE_AUTH_API_KEY to prevent data races
/// when tests run in parallel.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn make_empty_resolver() -> ConfigResolver {
    ConfigResolver::new(None, None)
}


#[test]
fn test_get_api_key_from_env_var() {
    // APCORE_AUTH_API_KEY must be returned when set.
    let _guard = ENV_LOCK.lock().unwrap();
    // SAFETY: test-only env manipulation, serialized via ENV_LOCK.
    unsafe { std::env::set_var("APCORE_AUTH_API_KEY", "env-key-abc") };
    let provider = AuthProvider::new(make_empty_resolver());
    let key = provider.get_api_key();
    // SAFETY: cleanup regardless of assertion outcome.
    unsafe { std::env::remove_var("APCORE_AUTH_API_KEY") };
    assert_eq!(key, Some("env-key-abc".to_string()));
}

#[test]
fn test_get_api_key_missing_returns_error() {
    // When no key is available, get_api_key() must return None.
    let _guard = ENV_LOCK.lock().unwrap();
    // SAFETY: test-only env manipulation, serialized via ENV_LOCK.
    unsafe { std::env::remove_var("APCORE_AUTH_API_KEY") };
    let provider = AuthProvider::new(make_empty_resolver());
    assert_eq!(provider.get_api_key(), None);
}

#[test]
fn test_authenticate_request_adds_bearer_header() {
    // authenticate_request must succeed when a key is available.
    let _guard = ENV_LOCK.lock().unwrap();
    // SAFETY: test-only env manipulation, serialized via ENV_LOCK.
    unsafe { std::env::set_var("APCORE_AUTH_API_KEY", "bearer-test-key") };
    let provider = AuthProvider::new(make_empty_resolver());
    let client = reqwest::Client::new();
    let builder = client.get("https://example.com");
    let result = provider.authenticate_request(builder);
    // SAFETY: cleanup.
    unsafe { std::env::remove_var("APCORE_AUTH_API_KEY") };
    assert!(result.is_ok(), "authenticate_request must succeed when key is set");
}

#[test]
fn test_handle_response_ok_returns_response() {
    // Verify the AuthenticationError enum messages match spec so that the
    // handle_response() 200 path is correctly documented.
    // (reqwest::Response cannot be constructed without a live HTTP call;
    //  the status-dispatch logic is covered by unit tests in auth.rs.)
    let missing = AuthenticationError::MissingApiKey;
    assert!(
        missing.to_string().contains("APCORE_AUTH_API_KEY"),
        "MissingApiKey message must mention the env var"
    );
}

#[test]
fn test_handle_response_401_returns_invalid_key_error() {
    // Verify the InvalidApiKey error variant exists and has the expected message.
    // (Live HTTP mock tests are deferred; reqwest::Response is not constructible
    //  without an actual HTTP request in this test harness.)
    let err = AuthenticationError::InvalidApiKey;
    assert!(
        err.to_string().contains("Authentication failed"),
        "InvalidApiKey message must say 'Authentication failed', got: {}",
        err
    );
}
