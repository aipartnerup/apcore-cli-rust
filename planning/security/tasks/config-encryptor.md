# Task: config-encryptor

**Feature**: security (FE-05)
**Status**: pending
**Estimated Time**: ~2 hours
**Depends On**: —
**Required By**: `auth`, `integration`

---

## Goal

Replace all `todo!()` stubs in `src/security/config_encryptor.rs` with a complete, spec-compliant `ConfigEncryptor`. After this task:

- `store()` persists to OS keyring when available, else falls back to AES-256-GCM file encryption.
- `retrieve()` decodes `keyring:…`, `enc:…`, and plaintext config values.
- `_keyring_available()` correctly identifies headless/CI environments.
- `_derive_key()` produces a 32-byte key via PBKDF2-HMAC-SHA256 with the machine-specific `hostname:username` material.
- `_aes_encrypt()` / `_aes_decrypt()` implement the wire format: `nonce[12] ‖ tag[16] ‖ ciphertext`.
- All inline `#[cfg(test)]` unit tests pass.

Also add `chrono` to `Cargo.toml` (needed by `audit` task); do it here to avoid churn later.

---

## Files Involved

| File | Action |
|---|---|
| `src/security/config_encryptor.rs` | Modify — implement all stubs |
| `Cargo.toml` | Modify — add `chrono = { version = "0.4", features = ["serde"] }` |

---

## Steps

### 1. Write failing unit tests first (TDD — RED)

Replace the three `assert!(false, "not implemented")` stubs in `src/security/config_encryptor.rs` with real assertions that fail for the right reason (missing implementation):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Helper: build an encryptor that always uses the AES path (keyring skipped).
    fn aes_encryptor() -> ConfigEncryptor {
        ConfigEncryptor { _force_aes: true }
    }

    #[test]
    fn test_aes_roundtrip() {
        // Encrypt then decrypt must recover the original plaintext.
        let enc = aes_encryptor();
        let ciphertext = enc._aes_encrypt("hello-secret").expect("encrypt");
        let plaintext  = enc._aes_decrypt(&ciphertext).expect("decrypt");
        assert_eq!(plaintext, "hello-secret");
    }

    #[test]
    fn test_store_without_keyring_returns_enc_prefix() {
        let enc = aes_encryptor();
        let token = enc.store("auth.api_key", "secret123").expect("store");
        assert!(token.starts_with("enc:"), "expected enc: prefix, got {token}");
    }

    #[test]
    fn test_retrieve_enc_value() {
        let enc = aes_encryptor();
        let token = enc.store("auth.api_key", "secret123").expect("store");
        let result = enc.retrieve(&token, "auth.api_key").expect("retrieve");
        assert_eq!(result, "secret123");
    }

    #[test]
    fn test_retrieve_plaintext_passthrough() {
        let enc = aes_encryptor();
        let result = enc.retrieve("plain-value", "some.key").expect("retrieve");
        assert_eq!(result, "plain-value");
    }

    #[test]
    fn test_retrieve_corrupted_ciphertext_returns_error() {
        let enc = aes_encryptor();
        // 28 bytes minimum: 12 nonce + 16 tag; pad with zeroes then corrupt tag.
        let mut bad = vec![0u8; 40];
        bad[12] ^= 0xFF; // corrupt tag byte
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bad);
        let config_value = format!("enc:{b64}");
        let result = enc.retrieve(&config_value, "some.key");
        assert!(matches!(result, Err(ConfigDecryptionError::AuthTagMismatch)));
    }

    #[test]
    fn test_retrieve_short_ciphertext_returns_error() {
        let enc = aes_encryptor();
        // Fewer than 28 bytes — missing nonce+tag.
        let b64 = base64::engine::general_purpose::STANDARD.encode(&[0u8; 10]);
        let config_value = format!("enc:{b64}");
        let result = enc.retrieve(&config_value, "some.key");
        assert!(matches!(result, Err(ConfigDecryptionError::AuthTagMismatch)));
    }

    #[test]
    fn test_derive_key_is_32_bytes() {
        let enc = aes_encryptor();
        let key = enc._derive_key().expect("derive");
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_nonces_are_unique() {
        // Each encrypt call must produce a different nonce (probabilistically).
        let enc = aes_encryptor();
        let ct1 = enc._aes_encrypt("same").expect("e1");
        let ct2 = enc._aes_encrypt("same").expect("e2");
        assert_ne!(&ct1[..12], &ct2[..12], "nonces must differ");
    }
}
```

Run to confirm RED:
```bash
cargo test --lib security::config_encryptor 2>&1 | grep -E "FAILED|error\[|^test "
```

### 2. Add `_force_aes` field and update `new()`

The struct needs a private `_force_aes: bool` field so tests can bypass the keyring probe:

```rust
pub struct ConfigEncryptor {
    _force_aes: bool,
}

impl Default for ConfigEncryptor {
    fn default() -> Self {
        Self { _force_aes: false }
    }
}

impl ConfigEncryptor {
    pub fn new() -> Result<Self, ConfigDecryptionError> {
        Ok(Self::default())
    }
```

### 3. Implement `_keyring_available()`

```rust
fn _keyring_available(&self) -> bool {
    if self._force_aes {
        return false;
    }
    // Attempt a sentinel get. If the entry is absent that is fine — it means
    // the keyring is accessible. Any other error means it is not.
    let entry = match keyring::Entry::new(Self::SERVICE_NAME, "__apcore_probe__") {
        Ok(e) => e,
        Err(_) => return false,
    };
    match entry.get_password() {
        Ok(_) | Err(keyring::Error::NoEntry) => true,
        Err(_) => false,
    }
}
```

### 4. Implement `_derive_key()`

```rust
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

fn _derive_key(&self) -> Result<[u8; 32], ConfigDecryptionError> {
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let salt: &[u8] = b"apcore-cli-config-v1";
    let material = format!("{hostname}:{username}");
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(material.as_bytes(), salt, 100_000, &mut key);
    Ok(key)
}
```

Note: `hostname` needs to be obtained via `gethostname` syscall. Use
`std::process::Command::new("hostname").output()` or add the `gethostname`
crate (tiny, no build script). Prefer the `gethostname` crate:

```toml
# Cargo.toml
gethostname = "0.4"
```

Then:
```rust
use gethostname::gethostname;
let hostname = gethostname().into_string().unwrap_or_else(|_| "unknown".to_string());
```

### 5. Implement `_aes_encrypt()` and `_aes_decrypt()`

```rust
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};

fn _aes_encrypt(&self, plaintext: &str) -> Result<Vec<u8>, ConfigDecryptionError> {
    let raw_key = self._derive_key()?;
    let cipher = Aes256Gcm::new_from_slice(&raw_key)
        .map_err(|e| ConfigDecryptionError::KdfError(e.to_string()))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 12-byte nonce
    // aes_gcm appends the 16-byte tag to the ciphertext in its output.
    // We need: nonce[12] + tag[16] + ciphertext
    // aes_gcm::encrypt returns ct||tag (tag is last 16 bytes).
    let encrypted = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|_| ConfigDecryptionError::AuthTagMismatch)?;
    // encrypted = ciphertext_bytes || tag[16]
    // Reorder to wire format: nonce || tag || ciphertext
    let ct_len = encrypted.len() - 16;
    let ciphertext = &encrypted[..ct_len];
    let tag = &encrypted[ct_len..];
    let mut out = Vec::with_capacity(12 + 16 + ct_len);
    out.extend_from_slice(nonce.as_slice());
    out.extend_from_slice(tag);
    out.extend_from_slice(ciphertext);
    Ok(out)
}

fn _aes_decrypt(&self, data: &[u8]) -> Result<String, ConfigDecryptionError> {
    if data.len() < 28 {
        return Err(ConfigDecryptionError::AuthTagMismatch);
    }
    let raw_key = self._derive_key()?;
    let cipher = Aes256Gcm::new_from_slice(&raw_key)
        .map_err(|e| ConfigDecryptionError::KdfError(e.to_string()))?;
    let nonce = Nonce::from_slice(&data[..12]);
    let tag = &data[12..28];
    let ciphertext = &data[28..];
    // aes_gcm::decrypt expects ciphertext || tag
    let mut ct_with_tag = Vec::with_capacity(ciphertext.len() + 16);
    ct_with_tag.extend_from_slice(ciphertext);
    ct_with_tag.extend_from_slice(tag);
    let plaintext = cipher
        .decrypt(nonce, ct_with_tag.as_slice())
        .map_err(|_| ConfigDecryptionError::AuthTagMismatch)?;
    String::from_utf8(plaintext).map_err(|_| ConfigDecryptionError::InvalidUtf8)
}
```

### 6. Implement `store()`

```rust
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

pub fn store(&self, key: &str, value: &str) -> Result<String, ConfigDecryptionError> {
    if self._keyring_available() {
        let entry = keyring::Entry::new(Self::SERVICE_NAME, key)
            .map_err(|e| ConfigDecryptionError::KeyringError(e.to_string()))?;
        entry.set_password(value)
            .map_err(|e| ConfigDecryptionError::KeyringError(e.to_string()))?;
        Ok(format!("keyring:{key}"))
    } else {
        tracing::warn!("OS keyring unavailable. Using file-based encryption.");
        let ciphertext = self._aes_encrypt(value)?;
        Ok(format!("enc:{}", B64.encode(&ciphertext)))
    }
}
```

### 7. Implement `retrieve()`

```rust
pub fn retrieve(&self, config_value: &str, key: &str) -> Result<String, ConfigDecryptionError> {
    if let Some(ref_key) = config_value.strip_prefix("keyring:") {
        let entry = keyring::Entry::new(Self::SERVICE_NAME, ref_key)
            .map_err(|e| ConfigDecryptionError::KeyringError(e.to_string()))?;
        entry.get_password().map_err(|e| match e {
            keyring::Error::NoEntry => ConfigDecryptionError::KeyringError(
                format!("Keyring entry not found for '{ref_key}'."),
            ),
            other => ConfigDecryptionError::KeyringError(other.to_string()),
        })
    } else if let Some(b64_data) = config_value.strip_prefix("enc:") {
        let data = B64.decode(b64_data)
            .map_err(|_| ConfigDecryptionError::AuthTagMismatch)?;
        self._aes_decrypt(&data).map_err(|_| {
            ConfigDecryptionError::KeyringError(format!(
                "Failed to decrypt configuration value '{key}'. \
                 Re-configure with 'apcore-cli config set {key}'."
            ))
        })
    } else {
        Ok(config_value.to_string())
    }
}
```

### 8. Add `base64` dependency

```toml
# Cargo.toml [dependencies]
base64 = "0.22"
```

### 9. Run tests (GREEN)

```bash
cargo test --lib security::config_encryptor 2>&1 | grep -E "^test |FAILED|error\["
```

All seven inline tests must pass.

### 10. Refactor and clippy

```bash
cargo clippy -- -D warnings 2>&1 | head -40
```

Fix any warnings. Ensure no `#[allow(dead_code)]` remains.

---

## Acceptance Criteria

- [ ] No `todo!()` macros remain in `src/security/config_encryptor.rs`
- [ ] `test_aes_roundtrip` passes: encrypt → decrypt recovers original plaintext
- [ ] `test_store_without_keyring_returns_enc_prefix` passes
- [ ] `test_retrieve_enc_value` passes: stored value is retrievable
- [ ] `test_retrieve_plaintext_passthrough` passes: plain string returned unchanged
- [ ] `test_retrieve_corrupted_ciphertext_returns_error` passes: `AuthTagMismatch`
- [ ] `test_retrieve_short_ciphertext_returns_error` passes: `AuthTagMismatch`
- [ ] `test_derive_key_is_32_bytes` passes
- [ ] `test_nonces_are_unique` passes
- [ ] Wire format is `nonce[12] ‖ tag[16] ‖ ciphertext`, base64-encoded under `enc:` prefix
- [ ] PBKDF2 uses SHA-256, salt `b"apcore-cli-config-v1"`, 100 000 iterations
- [ ] `cargo clippy -- -D warnings` clean in this file
- [ ] `chrono` and `base64` added to `Cargo.toml`

---

## Dependencies

- **Depends on**: — (no prior task)
- **Required by**: `auth`, `integration`
