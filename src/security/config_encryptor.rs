// apcore-cli — Encrypted config storage.
// Protocol spec: SEC-03 (ConfigEncryptor, ConfigDecryptionError)

use thiserror::Error;

// ---------------------------------------------------------------------------
// ConfigDecryptionError
// ---------------------------------------------------------------------------

/// Errors produced by decryption or key-derivation operations.
#[derive(Debug, Error)]
pub enum ConfigDecryptionError {
    /// The ciphertext is malformed or has been tampered with.
    #[error("decryption failed: authentication tag mismatch or corrupt data")]
    AuthTagMismatch,

    /// The stored data was not valid UTF-8 after decryption.
    #[error("decrypted data is not valid UTF-8")]
    InvalidUtf8,

    /// Keyring access failed.
    #[error("keyring error: {0}")]
    KeyringError(String),

    /// Key-derivation failed.
    #[error("key derivation error: {0}")]
    KdfError(String),
}

// ---------------------------------------------------------------------------
// ConfigEncryptor
// ---------------------------------------------------------------------------

/// AES-GCM encrypted config store backed by the system keyring.
///
/// Uses PBKDF2-HMAC-SHA256 for key derivation from a passphrase or machine
/// secret, and AES-256-GCM for authenticated encryption.
pub struct ConfigEncryptor {
    // TODO: hold encryption key material (derived at construction time)
}

impl ConfigEncryptor {
    /// Create a new `ConfigEncryptor` and derive the encryption key.
    ///
    /// The key is derived from the OS hostname + a stored salt retrieved from
    /// the keyring, so it is machine-specific.
    pub fn new() -> Result<Self, ConfigDecryptionError> {
        // TODO: retrieve or generate salt from keyring, run PBKDF2, store key.
        todo!("ConfigEncryptor::new")
    }

    /// Encrypt and persist a config value to the system keyring.
    ///
    /// # Arguments
    /// * `service` — logical name for the credential (e.g. `"apcore-cli.api_key"`)
    /// * `value`   — plaintext value to store
    pub fn store(&self, service: &str, value: &str) -> Result<(), ConfigDecryptionError> {
        // TODO: generate random nonce, encrypt with AES-256-GCM, base64-encode,
        //       store via keyring crate.
        let _ = (service, value);
        todo!("ConfigEncryptor::store")
    }

    /// Retrieve and decrypt a config value from the system keyring.
    ///
    /// # Arguments
    /// * `service` — logical name for the credential
    ///
    /// # Errors
    /// * `ConfigDecryptionError::KeyringError` — entry not found
    /// * `ConfigDecryptionError::AuthTagMismatch` — decryption failed
    pub fn retrieve(&self, service: &str) -> Result<String, ConfigDecryptionError> {
        // TODO: fetch base64 ciphertext from keyring, decode, decrypt, return.
        let _ = service;
        todo!("ConfigEncryptor::retrieve")
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve_roundtrip() {
        // store() then retrieve() must return the original plaintext.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_retrieve_missing_key_returns_error() {
        // Retrieving a non-existent key must yield KeyringError.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_tampered_ciphertext_returns_auth_tag_error() {
        // Modifying the ciphertext must yield AuthTagMismatch on retrieve().
        assert!(false, "not implemented");
    }
}
