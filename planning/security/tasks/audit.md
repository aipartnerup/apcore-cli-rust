# Task: audit

**Feature**: security (FE-05)
**Status**: pending
**Estimated Time**: ~1 hour
**Depends On**: —
**Required By**: `integration`

---

## Goal

Replace all `todo!()` stubs in `src/security/audit.rs` with a complete `AuditLogger`. After this task:

- `AuditLogger::new(path)` creates parent directories eagerly; `path = None` means use `~/.apcore-cli/audit.jsonl`.
- `log_execution()` appends a single JSONL line with all required fields; IO failures produce `tracing::warn!` only — never panic or propagate an error.
- `_get_user()` uses `USER` → `LOGNAME` → `"unknown"` fallback (no external crate).
- `_hash_input()` generates a fresh 16-byte random salt per call and SHA-256-hashes `salt ‖ JSON(input_data, sort_keys)`.
- All inline `#[cfg(test)]` unit tests pass.

---

## Files Involved

| File | Action |
|---|---|
| `src/security/audit.rs` | Modify — implement all stubs |
| `Cargo.toml` | Verify `chrono` is present (added in `config-encryptor` task); if not, add it here |

---

## Steps

### 1. Revise the struct and constructor (RED)

The existing stub has `path: Option<PathBuf>`. Per spec the default path is
`~/.apcore-cli/audit.jsonl`. Update the struct and fix the inline tests to fail
for the right reason:

```rust
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use chrono::Utc;
use sha2::{Digest, Sha256};
use serde_json::{json, Value};

pub struct AuditLogger {
    path: Option<PathBuf>,
}

impl AuditLogger {
    /// Default path: `~/.apcore-cli/audit.jsonl`.
    pub fn default_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".apcore-cli").join("audit.jsonl"))
    }

    pub fn new(path: Option<PathBuf>) -> Self {
        let resolved = path.or_else(Self::default_path);
        if let Some(ref p) = resolved {
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent); // best-effort
            }
        }
        Self { path: resolved }
    }
```

Replace the four `assert!(false, "not implemented")` stubs:

```rust
    #[test]
    fn test_audit_logger_disabled_no_op() {
        // AuditLogger with path=None must not write any files.
        // Override: pass a None but also ensure default_path() returns None
        // in test context by supplying an explicit impossible path.
        // Simplest: pass Some(path) to a read-only dir and expect warn, not panic.
        let logger = AuditLogger { path: None };
        // Should not panic even with no path.
        logger.log_execution("mod.test", &json!({}), "success", 0, 1);
    }

    #[test]
    fn test_audit_logger_writes_jsonl_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let logger = AuditLogger::new(Some(path.clone()));
        logger.log_execution("math.add", &json!({"a": 1}), "success", 0, 42);
        let content = std::fs::read_to_string(&path).unwrap();
        let entry: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(entry["module_id"], "math.add");
        assert_eq!(entry["status"], "success");
        assert_eq!(entry["exit_code"], 0);
        assert_eq!(entry["duration_ms"], 42);
    }

    #[test]
    fn test_audit_logger_appends_multiple_records() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let logger = AuditLogger::new(Some(path.clone()));
        logger.log_execution("a.b", &json!({}), "success", 0, 1);
        logger.log_execution("c.d", &json!({}), "error", 1, 2);
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_audit_logger_record_contains_required_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let logger = AuditLogger::new(Some(path.clone()));
        logger.log_execution("x.y", &json!({"k": "v"}), "success", 0, 10);
        let raw = std::fs::read_to_string(&path).unwrap();
        let entry: serde_json::Value = serde_json::from_str(raw.trim()).unwrap();
        assert!(entry["timestamp"].as_str().unwrap().ends_with('Z'));
        assert!(entry["user"].is_string());
        assert_eq!(entry["module_id"], "x.y");
        assert!(entry["input_hash"].as_str().unwrap().len() == 64); // hex SHA-256
        assert_eq!(entry["status"], "success");
        assert!(entry["exit_code"].is_number());
        assert!(entry["duration_ms"].is_number());
    }
```

Run to confirm RED:
```bash
cargo test --lib security::audit 2>&1 | grep -E "FAILED|error\[|^test "
```

### 2. Implement `_get_user()`

```rust
fn _get_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}
```

No platform-specific `getlogin()` equivalent is needed in Rust (no direct libc call required by spec). The Python lesson says `os.getlogin()` can fail; the Rust fallback chain above is already safe.

### 3. Implement `_hash_input()`

```rust
fn _hash_input(input_data: &Value) -> String {
    use rand::RngCore;
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    let payload = serde_json::to_string(input_data)
        .unwrap_or_else(|_| "{}".to_string());
    let mut hasher = Sha256::new();
    hasher.update(&salt);
    hasher.update(payload.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

`serde_json` does not guarantee sorted keys in `to_string`. Use `serde_json::to_string` on a `BTreeMap`-backed value or sort manually. The spec says `sort_keys=True` (Python). Correct approach:

```rust
fn _hash_input(input_data: &Value) -> String {
    // Stable JSON representation: sort object keys recursively.
    fn stable_json(v: &Value) -> String {
        match v {
            Value::Object(map) => {
                let sorted: std::collections::BTreeMap<_, _> = map.iter().collect();
                let pairs: String = sorted
                    .iter()
                    .map(|(k, v)| format!("{}:{}", serde_json::json!(k), stable_json(v)))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{{{pairs}}}")
            }
            other => other.to_string(),
        }
    }
    let mut salt = [0u8; 16];
    // Use OsRng for cryptographic salt (matches spec intent).
    use aes_gcm::aead::OsRng;
    use rand::RngCore;
    OsRng.fill_bytes(&mut salt);
    let payload = stable_json(input_data);
    let mut hasher = Sha256::new();
    hasher.update(&salt);
    hasher.update(payload.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

### 4. Implement `log_execution()`

```rust
pub fn log_execution(
    &self,
    module_id: &str,
    input_data: &Value,
    status: &str,        // "success" | "error"
    exit_code: i32,
    duration_ms: u64,
) {
    let Some(ref path) = self.path else {
        return; // logging disabled
    };

    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let entry = json!({
        "timestamp":   timestamp,
        "user":        Self::_get_user(),
        "module_id":   module_id,
        "input_hash":  Self::_hash_input(input_data),
        "status":      status,
        "exit_code":   exit_code,
        "duration_ms": duration_ms,
    });

    let result = (|| -> std::io::Result<()> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &entry)?;
        writeln!(writer)?;
        writer.flush()?;
        Ok(())
    })();

    if let Err(e) = result {
        tracing::warn!("Could not write audit log: {e}");
    }
}
```

Note: The existing stub has a different `log_execution` signature (`input: &Value, output: &Value`). Update the signature to match the spec (`input_data: &Value, status: &str, exit_code: i32, duration_ms: u64`). Update `mod.rs` re-export accordingly — since `AuditLogger` is re-exported by name only, no re-export change is needed.

### 5. Update `AuditLogError` (if needed)

The current `AuditLogError` enum is fine but no longer returned from `log_execution` (errors are swallowed and warned). Remove the `Result` return type from `log_execution` — it must be `pub fn log_execution(…) -> ()`. The `AuditLogError` type can remain for potential future use.

### 6. Run tests (GREEN)

```bash
cargo test --lib security::audit 2>&1 | grep -E "^test |FAILED|error\["
```

### 7. Refactor and clippy

```bash
cargo clippy -- -D warnings 2>&1 | head -40
```

---

## Acceptance Criteria

- [ ] No `todo!()` macros remain in `src/security/audit.rs`
- [ ] `test_audit_logger_disabled_no_op` passes (no panic when `path = None`)
- [ ] `test_audit_logger_writes_jsonl_record` passes: entry has correct `module_id`, `status`, `exit_code`, `duration_ms`
- [ ] `test_audit_logger_appends_multiple_records` passes: two calls produce two JSONL lines
- [ ] `test_audit_logger_record_contains_required_fields` passes: all seven fields present; `input_hash` is 64-char hex; timestamp ends with `Z`
- [ ] IO failure path does not panic; emits `tracing::warn!`
- [ ] `_get_user()` returns `USER` → `LOGNAME` → `"unknown"` without any external crate
- [ ] Input hash uses a fresh 16-byte random salt per invocation (nonces differ across calls)
- [ ] JSON keys are sorted before hashing (same input → same hash within one call; different invocations use different salts)
- [ ] Default path resolves to `~/.apcore-cli/audit.jsonl` when `path = None`
- [ ] Parent directory is created eagerly on construction (best-effort; failure is silent)
- [ ] `log_execution` return type is `()` — never returns `Err`
- [ ] `cargo clippy -- -D warnings` clean in this file

---

## Dependencies

- **Depends on**: — (no prior task; `chrono` added by `config-encryptor` or added here)
- **Required by**: `integration`
