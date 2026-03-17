# Task: collect-input

**Feature**: FE-01 Core Dispatcher
**File**: `src/cli.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~3h
**Depends on**: `validate-module-id` (must compile; no logical dep)
**Required by**: `lazy-module-group-skeleton`, `exec-dispatch-callback`

---

## Context

`collect_input` merges STDIN JSON with CLI flags. CLI flags win for duplicate keys. The current public signature in `cli.rs` accepts `Option<&str>` for the stdin flag, `HashMap<String, Value>` for CLI kwargs, and a `bool` for the large-input bypass.

The core difficulty is testability: `std::io::stdin()` cannot be easily injected in unit tests. The implementation must be refactored to accept a generic `impl Read` so tests can pass a `std::io::Cursor`.

The public function `collect_input` (called by dispatch code) wraps the inner generic function with `stdin()`.

---

## RED — Write Failing Tests First

Update `tests/test_cli.rs` stubs. The STDIN-dependent tests require a refactored inner function. Add the inner function to the public API or use a `#[cfg(test)]` helper.

```rust
// In tests/test_cli.rs — replace assert!(false) stubs:

use std::io::Cursor;
use apcore_cli::cli::{collect_input_from_reader, CliError};

#[test]
fn test_collect_input_no_stdin_drops_null_values() {
    let mut kwargs = HashMap::new();
    kwargs.insert("a".to_string(), json!(5));
    kwargs.insert("b".to_string(), Value::Null);

    let result = collect_input(None, kwargs, false).unwrap();
    assert_eq!(result.get("a"), Some(&json!(5)));
    assert!(!result.contains_key("b"), "Null values must be dropped");
}

#[test]
fn test_collect_input_stdin_valid_json() {
    let stdin_bytes = b"{\"x\": 42}";
    let reader = Cursor::new(stdin_bytes.to_vec());
    let result = collect_input_from_reader(Some("-"), HashMap::new(), false, reader).unwrap();
    assert_eq!(result.get("x"), Some(&json!(42)));
}

#[test]
fn test_collect_input_cli_overrides_stdin() {
    let stdin_bytes = b"{\"a\": 5}";
    let reader = Cursor::new(stdin_bytes.to_vec());
    let mut kwargs = HashMap::new();
    kwargs.insert("a".to_string(), json!(99));
    let result = collect_input_from_reader(Some("-"), kwargs, false, reader).unwrap();
    assert_eq!(result.get("a"), Some(&json!(99)), "CLI must override STDIN");
}

#[test]
fn test_collect_input_oversized_stdin_rejected() {
    let big = vec![b' '; 10 * 1024 * 1024 + 1];
    // Wrap in a minimal JSON object prefix/suffix to avoid JsonParse:
    // but size check happens before parse, so raw bytes suffice.
    let reader = Cursor::new(big);
    let err = collect_input_from_reader(Some("-"), HashMap::new(), false, reader).unwrap_err();
    assert!(matches!(err, CliError::InputTooLarge { .. }));
}

#[test]
fn test_collect_input_large_input_allowed() {
    // 11 MiB of valid JSON whitespace inside an object:
    let mut payload = b"{\"k\": \"".to_vec();
    payload.extend(vec![b'x'; 11 * 1024 * 1024]);
    payload.extend(b"\"}");
    let reader = Cursor::new(payload);
    let result = collect_input_from_reader(Some("-"), HashMap::new(), true, reader);
    assert!(result.is_ok(), "large_input=true must accept oversized payload");
}

#[test]
fn test_collect_input_invalid_json_returns_error() {
    let reader = Cursor::new(b"not json at all".to_vec());
    let err = collect_input_from_reader(Some("-"), HashMap::new(), false, reader).unwrap_err();
    assert!(matches!(err, CliError::JsonParse(_)));
}

#[test]
fn test_collect_input_non_object_json_returns_error() {
    let reader = Cursor::new(b"[1, 2, 3]".to_vec());
    let err = collect_input_from_reader(Some("-"), HashMap::new(), false, reader).unwrap_err();
    assert!(matches!(err, CliError::NotAnObject));
}

#[test]
fn test_collect_input_empty_stdin_returns_empty_map() {
    let reader = Cursor::new(b"".to_vec());
    let result = collect_input_from_reader(Some("-"), HashMap::new(), false, reader).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_collect_input_no_stdin_flag_returns_cli_kwargs() {
    let mut kwargs = HashMap::new();
    kwargs.insert("foo".to_string(), json!("bar"));
    let result = collect_input(None, kwargs.clone(), false).unwrap();
    assert_eq!(result.get("foo"), Some(&json!("bar")));
}
```

Run `cargo test collect_input` — all fail because `collect_input_from_reader` does not exist and `collect_input` panics.

---

## GREEN — Implement

Add `collect_input_from_reader` to `src/cli.rs` and wire `collect_input` to call it with `stdin()`.

```rust
use std::io::Read;

const DEFAULT_STDIN_LIMIT: usize = 10 * 1024 * 1024; // 10 MiB

/// Inner implementation: accepts any `Read` source for testability.
pub fn collect_input_from_reader<R: Read>(
    stdin_flag: Option<&str>,
    cli_kwargs: HashMap<String, Value>,
    large_input: bool,
    mut reader: R,
) -> Result<HashMap<String, Value>, CliError> {
    // Drop Null values from CLI kwargs (mirrors Python's {k:v for k,v in ... if v is not None}).
    let cli_non_null: HashMap<String, Value> = cli_kwargs
        .into_iter()
        .filter(|(_, v)| !v.is_null())
        .collect();

    if stdin_flag != Some("-") {
        return Ok(cli_non_null);
    }

    // Read all bytes from the reader.
    let mut buf = Vec::new();
    reader
        .read_to_end(&mut buf)
        .map_err(|e| CliError::StdinRead(e.to_string()))?;

    // Enforce size limit.
    if !large_input && buf.len() > DEFAULT_STDIN_LIMIT {
        return Err(CliError::InputTooLarge {
            limit: DEFAULT_STDIN_LIMIT,
            actual: buf.len(),
        });
    }

    // Empty stdin → empty map.
    if buf.is_empty() {
        return Ok(cli_non_null);
    }

    // Parse as JSON.
    let stdin_value: Value = serde_json::from_slice(&buf)
        .map_err(|e| CliError::JsonParse(e.to_string()))?;

    // Must be a JSON object.
    let stdin_map = match stdin_value {
        Value::Object(m) => m,
        _ => return Err(CliError::NotAnObject),
    };

    // Merge: STDIN base, CLI flags override.
    let mut merged: HashMap<String, Value> = stdin_map.into_iter().collect();
    merged.extend(cli_non_null);
    Ok(merged)
}

/// Public API: reads from `std::io::stdin()`.
pub fn collect_input(
    stdin_flag: Option<&str>,
    cli_kwargs: HashMap<String, Value>,
    large_input: bool,
) -> Result<HashMap<String, Value>, CliError> {
    collect_input_from_reader(stdin_flag, cli_kwargs, large_input, std::io::stdin())
}
```

Export `collect_input_from_reader` from `lib.rs`:

```rust
pub use cli::{collect_input, collect_input_from_reader, ...};
```

Run `cargo test collect_input` — all pass.

---

## REFACTOR

- Replace the `DEFAULT_STDIN_LIMIT` constant that was already defined in the stub with the implementation (avoid duplicate definition).
- Confirm error messages match the spec exactly:
  - Oversized: `"Error: STDIN input exceeds 10MB limit. Use --large-input to override."`
  - Bad JSON: `"Error: STDIN does not contain valid JSON: {detail}."`
  - Not object: `"Error: STDIN JSON must be an object, got {type}."`

  These messages are emitted by the caller (the exec dispatch callback) using the `CliError` variant's display string plus the exact wording. Ensure `CliError` display strings align with the spec, or adjust the caller's formatting.

- Run `cargo clippy -- -D warnings`.

---

## Verification

```bash
cargo test collect_input 2>&1
# Expected: test result: ok. N passed; 0 failed
```

Integration path for T-DISP-06, T-DISP-07, T-DISP-08, T-DISP-09 is covered in `tests/test_e2e.rs`.
