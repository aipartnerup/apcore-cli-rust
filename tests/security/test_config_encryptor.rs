// apcore-cli — Integration tests for ConfigEncryptor.
// Protocol spec: SEC-03

use apcore_cli::security::config_encryptor::{ConfigDecryptionError, ConfigEncryptor};

#[test]
fn test_store_and_retrieve_roundtrip() {
    // store("svc", "val") then retrieve("svc") must return "val".
    // TODO: initialise ConfigEncryptor, store and retrieve, assert eq.
    assert!(false, "not implemented");
}

#[test]
fn test_retrieve_missing_key_returns_error() {
    // Retrieving a service that was never stored must yield KeyringError.
    assert!(false, "not implemented");
}

#[test]
fn test_tampered_ciphertext_returns_auth_tag_error() {
    // Corrupting the stored ciphertext must yield AuthTagMismatch on retrieve.
    assert!(false, "not implemented");
}

#[test]
fn test_different_services_are_independent() {
    // Values stored under different service names must not interfere.
    assert!(false, "not implemented");
}
