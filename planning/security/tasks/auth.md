# Task: auth

**Feature**: security (FE-05)
**Status**: pending
**Estimated Time**: ~1 hour
**Depends On**: `config-encryptor`
**Required By**: `integration`

---

## Goal

Replace all `todo!()` stubs in `src/security/auth.rs` with a complete `AuthProvider`. After this task:

- `get_api_key()` resolves via: env `APCORE_AUTH_API_KEY` → `keyring:`/`enc:` config value (via `ConfigEncryptor::retrieve`) → config file `auth.api_key` → `None`.
- `authenticate_request(builder)` injects `Authorization: Bearer {key}` or returns `AuthenticationError::MissingApiKey`.
- `handle_response(response)` maps HTTP 401/403 → `AuthenticationError::InvalidApiKey`.
- All inline `#[cfg(test)]` unit tests pass.

---

## Files Involved

| File | Action |
|---|---|
| `src/security/auth.rs` | Modify — implement all stubs |

---

## Steps

### 1. Write failing unit tests first (TDD — RED)

Replace the four `assert!(false, "not implemented")` stubs:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_resolver_with_key(key: &str) -> ConfigResolver {
        // Build a ConfigResolver that returns `key` for "auth.api_key".
        // Use the cli_flags path so it does not touch the filesystem.
        let mut flags = std::collections::HashMap::new();
        flags.insert("--api-key".to_string(), Some(key.to_string()));
        ConfigResolver::new(Some(flags), None)
    }

    fn make_resolver_empty() -> ConfigResolver {
        ConfigResolver::new(None, None)
    }

    #[test]
    fn test_get_api_key_from_env_var() {
        // SAFETY: test-only env manipulation; run serially if needed.
        unsafe { std::env::set_var("APCORE_AUTH_API_KEY", "test-key-env") };
        let provider = AuthProvider::new(make_resolver_empty());
        let result = provider.get_api_key();
        assert_eq!(result, Some("test-key-env".to_string()));
        unsafe { std::env::remove_var("APCORE_AUTH_API_KEY") };
    }

    #[test]
    fn test_get_api_key_none_when_not_configured() {
        unsafe { std::env::remove_var("APCORE_AUTH_API_KEY") };
        let provider = AuthProvider::new(make_resolver_empty());
        let result = provider.get_api_key();
        assert_eq!(result, None);
    }

    #[test]
    fn test_authenticate_request_adds_bearer_header() {
        unsafe { std::env::set_var("APCORE_AUTH_API_KEY", "abc123") };
        let provider = AuthProvider::new(make_resolver_empty());
        let client = reqwest::Client::new();
        let builder = client.get("https://example.com");
        let result = provider.authenticate_request(builder);
        assert!(result.is_ok());
        unsafe { std::env::remove_var("APCORE_AUTH_API_KEY") };
    }

    #[test]
    fn test_authenticate_request_no_key_raises() {
        unsafe { std::env::remove_var("APCORE_AUTH_API_KEY") };
        let provider = AuthProvider::new(make_resolver_empty());
        let client = reqwest::Client::new();
        let builder = client.get("https://example.com");
        let result = provider.authenticate_request(builder);
        assert!(matches!(result, Err(AuthenticationError::MissingApiKey)));
    }
}
```

Run to confirm RED:
```bash
cargo test --lib security::auth 2>&1 | grep -E "FAILED|error\[|^test "
```

### 2. Implement `get_api_key()`

Resolution order per spec (FR-SEC-001):
1. `APCORE_AUTH_API_KEY` env var.
2. Config resolver `auth.api_key` field (may be `keyring:…` or `enc:…` — decode via `ConfigEncryptor`).
3. Return `None` if neither is present.

```rust
pub fn get_api_key(&self) -> Option<String> {
    // Tier 1: environment variable (plain value — pass through as-is).
    if let Ok(val) = std::env::var("APCORE_AUTH_API_KEY") {
        if !val.is_empty() {
            return Some(val);
        }
    }

    // Tier 2: config file via resolver.
    let raw = self.config.resolve("auth.api_key", Some("--api-key"), Some("APCORE_AUTH_API_KEY"))?;

    // If the stored value is a keyring ref or enc blob, decode it.
    if raw.starts_with("keyring:") || raw.starts_with("enc:") {
        let encryptor = ConfigEncryptor::new().ok()?;
        encryptor.retrieve(&raw, "auth.api_key").ok()
    } else {
        Some(raw)
    }
}
```

Note: `get_api_key()` returns `Option<String>`, not `Result`. Errors in decryption are swallowed here (caller will surface `MissingApiKey`). For audit-grade error visibility, log a warning before returning `None`:

```rust
        encryptor.retrieve(&raw, "auth.api_key").map_err(|e| {
            tracing::warn!("Failed to decode auth.api_key: {e}");
        }).ok()
```

### 3. Implement `authenticate_request()`

```rust
pub fn authenticate_request(
    &self,
    builder: reqwest::RequestBuilder,
) -> Result<reqwest::RequestBuilder, AuthenticationError> {
    let key = self.get_api_key().ok_or(AuthenticationError::MissingApiKey)?;
    Ok(builder.header("Authorization", format!("Bearer {key}")))
}
```

The error message on `MissingApiKey` is defined in the `thiserror` `#[error(…)]` attribute on the enum variant. Per spec: "Remote registry requires authentication. Set --api-key, APCORE_AUTH_API_KEY, or auth.api_key in config." Update the variant message in the error enum to match exactly:

```rust
#[error(
    "Remote registry requires authentication. \
     Set --api-key, APCORE_AUTH_API_KEY, or auth.api_key in config."
)]
MissingApiKey,
```

### 4. Implement `handle_response()`

```rust
pub fn handle_response(
    &self,
    response: reqwest::Response,
) -> Result<reqwest::Response, AuthenticationError> {
    match response.status().as_u16() {
        401 | 403 => Err(AuthenticationError::InvalidApiKey),
        _ => Ok(response),
    }
}
```

Update `InvalidApiKey` error message to match spec exactly:
```rust
#[error("Authentication failed. Verify your API key.")]
InvalidApiKey,
```

### 5. Run tests (GREEN)

```bash
cargo test --lib security::auth 2>&1 | grep -E "^test |FAILED|error\["
```

All four inline tests must pass.

### 6. Refactor and clippy

```bash
cargo clippy -- -D warnings 2>&1 | head -40
```

---

## Acceptance Criteria

- [ ] No `todo!()` macros remain in `src/security/auth.rs`
- [ ] `test_get_api_key_from_env_var` passes
- [ ] `test_get_api_key_none_when_not_configured` passes
- [ ] `test_authenticate_request_adds_bearer_header` passes
- [ ] `test_authenticate_request_no_key_raises` returns `AuthenticationError::MissingApiKey`
- [ ] `handle_response` with status 401 returns `AuthenticationError::InvalidApiKey`
- [ ] `handle_response` with status 403 returns `AuthenticationError::InvalidApiKey`
- [ ] `handle_response` with status 200 returns `Ok(response)`
- [ ] `MissingApiKey` error message matches spec exactly (exit 77)
- [ ] `InvalidApiKey` error message matches spec exactly (exit 77)
- [ ] `cargo clippy -- -D warnings` clean in this file

---

## Dependencies

- **Depends on**: `config-encryptor` (`ConfigEncryptor::new()` and `retrieve()` must be implemented)
- **Required by**: `integration`
