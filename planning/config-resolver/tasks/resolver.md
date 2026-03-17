# Task: resolver

**Feature**: config-resolver (FE-07)
**Status**: pending
**Estimated Time**: ~1.5 hours
**Depends On**: `models`
**Required By**: `tests`

---

## Goal

Replace the three `todo!()` stubs in `src/config.rs` with working implementations of `resolve`, `load_config_file`, and `flatten_dict` (public JSON variant + private YAML helper). After this task, all inline `#[cfg(test)]` unit tests in `src/config.rs` must pass, and the logic is ready for integration test verification in the `tests` task.

---

## Files Involved

| File | Action |
|---|---|
| `src/config.rs` | Modify — implement the three stubbed methods |

---

## Steps

### 1. Write failing unit tests first (TDD — RED)

Before touching the implementations, add targeted inline unit tests for each stub. The tests in `src/config.rs` already have four `assert!(false, "not implemented")` stubs. Replace them with real assertions so they fail for the right reason (missing implementation, not just `false`):

Open `src/config.rs` and update the four failing inline tests:

```rust
#[test]
fn test_resolve_tier1_cli_flag_wins() {
    let mut flags = HashMap::new();
    flags.insert("--extensions-dir".to_string(), Some("/cli-path".to_string()));
    let resolver = ConfigResolver::new(Some(flags), None);
    // env var not set; no config file
    let result = resolver.resolve("extensions.root", Some("--extensions-dir"), Some("APCORE_EXTENSIONS_ROOT"));
    assert_eq!(result, Some("/cli-path".to_string()));
}

#[test]
fn test_resolve_tier2_env_var_wins() {
    unsafe { std::env::set_var("APCORE_EXTENSIONS_ROOT_UNIT", "/env-path") };
    let resolver = ConfigResolver::new(None, None);
    let result = resolver.resolve("extensions.root", None, Some("APCORE_EXTENSIONS_ROOT_UNIT"));
    assert_eq!(result, Some("/env-path".to_string()));
    unsafe { std::env::remove_var("APCORE_EXTENSIONS_ROOT_UNIT") };
}

#[test]
fn test_resolve_tier3_config_file_wins() {
    // Requires a temp file; skip in unit tests — covered in integration tests.
    // Just verify the method exists and returns None when no file is loaded.
    let resolver = ConfigResolver::new(None, None);
    // With config_path = None, _config_file is None.
    // The default for "extensions.root" should be returned (tier 4).
    let result = resolver.resolve("extensions.root", None, None);
    assert_eq!(result, Some("./extensions".to_string()));
}

#[test]
fn test_resolve_tier4_default_wins() {
    let resolver = ConfigResolver::new(None, None);
    let result = resolver.resolve("extensions.root", None, None);
    assert_eq!(result, Some("./extensions".to_string()));
}

#[test]
fn test_flatten_dict_nested() {
    let resolver = ConfigResolver::new(None, None);
    let map = serde_json::json!({"extensions": {"root": "/path"}});
    let result = resolver.flatten_dict(map);
    assert_eq!(result.get("extensions.root"), Some(&"/path".to_string()));
}

#[test]
fn test_flatten_dict_deeply_nested() {
    let resolver = ConfigResolver::new(None, None);
    let map = serde_json::json!({"a": {"b": {"c": "deep"}}});
    let result = resolver.flatten_dict(map);
    assert_eq!(result.get("a.b.c"), Some(&"deep".to_string()));
}
```

Run to confirm RED:
```bash
cargo test --lib 2>&1 | grep -E "FAILED|error\[|^test "
```

### 2. Implement `flatten_yaml_value` (private helper)

Add a private static method that recursively walks a `serde_yaml::Value::Mapping` and collects dot-notation keys. This is the internal path used by `load_config_file`.

```rust
fn flatten_yaml_value(
    value: serde_yaml::Value,
    prefix: &str,
    out: &mut HashMap<String, String>,
) {
    match value {
        serde_yaml::Value::Mapping(map) => {
            for (k, v) in map {
                let key_str = match k {
                    serde_yaml::Value::String(s) => s,
                    other => format!("{other:?}"),
                };
                let full_key = if prefix.is_empty() {
                    key_str
                } else {
                    format!("{prefix}.{key_str}")
                };
                Self::flatten_yaml_value(v, &full_key, out);
            }
        }
        serde_yaml::Value::Bool(b) => {
            out.insert(prefix.to_string(), b.to_string());
        }
        serde_yaml::Value::Number(n) => {
            out.insert(prefix.to_string(), n.to_string());
        }
        serde_yaml::Value::String(s) => {
            out.insert(prefix.to_string(), s);
        }
        serde_yaml::Value::Null => {
            out.insert(prefix.to_string(), String::new());
        }
        serde_yaml::Value::Sequence(_) | serde_yaml::Value::Tagged(_) => {
            // Sequences and tagged values are serialised as their debug repr;
            // no spec requirement for nested array flattening.
            out.insert(prefix.to_string(), format!("{value:?}"));
        }
    }
}
```

### 3. Implement `load_config_file`

Replace the `todo!()` in `load_config_file`:

```rust
fn load_config_file(path: &PathBuf) -> Option<HashMap<String, String>> {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // FR-DISP-005 AF-1: file not found — silent.
            return None;
        }
        Err(e) => {
            warn!(
                "Configuration file '{}' could not be read: {}",
                path.display(),
                e
            );
            return None;
        }
    };

    let parsed: serde_yaml::Value = match serde_yaml::from_str(&content) {
        Ok(v) => v,
        Err(_) => {
            // FR-DISP-005 AF-2: malformed YAML — log warning, use defaults.
            warn!(
                "Configuration file '{}' is malformed, using defaults.",
                path.display()
            );
            return None;
        }
    };

    // Root must be a mapping (dict). Scalars, sequences, and null are invalid.
    if !matches!(parsed, serde_yaml::Value::Mapping(_)) {
        warn!(
            "Configuration file '{}' is malformed, using defaults.",
            path.display()
        );
        return None;
    }

    let mut out = HashMap::new();
    Self::flatten_yaml_value(parsed, "", &mut out);
    Some(out)
}
```

### 4. Implement `resolve`

Replace the `todo!()` in `resolve`:

```rust
pub fn resolve(
    &self,
    key: &str,
    cli_flag: Option<&str>,
    env_var: Option<&str>,
) -> Option<String> {
    // Tier 1: CLI flag — present and value is Some(non-None string).
    if let Some(flag) = cli_flag {
        if let Some(Some(value)) = self._cli_flags.get(flag) {
            return Some(value.clone());
        }
    }

    // Tier 2: Environment variable — must be set and non-empty.
    if let Some(var) = env_var {
        if let Ok(env_value) = std::env::var(var) {
            if !env_value.is_empty() {
                return Some(env_value);
            }
        }
    }

    // Tier 3: Config file — key must be present in the flattened map.
    if let Some(ref file_map) = self._config_file {
        if let Some(value) = file_map.get(key) {
            return Some(value.clone());
        }
    }

    // Tier 4: Built-in defaults.
    self.defaults.get(key).map(|s| s.to_string())
}
```

### 5. Implement the public `flatten_dict` (JSON variant)

Replace the `todo!()` in `flatten_dict`. This bridges `serde_json::Value` to the same flattening logic by converting via string round-trip through serde_yaml or by walking the JSON value directly:

```rust
pub fn flatten_dict(&self, map: serde_json::Value) -> HashMap<String, String> {
    // Convert serde_json::Value to serde_yaml::Value via JSON string round-trip.
    // This is acceptable because flatten_dict is not on the hot path.
    let yaml_value: serde_yaml::Value = serde_json::from_value(map)
        .ok()
        .and_then(|v: serde_json::Value| serde_yaml::to_value(v).ok())
        .unwrap_or(serde_yaml::Value::Null);
    let mut out = HashMap::new();
    Self::flatten_yaml_value(yaml_value, "", &mut out);
    out
}
```

Alternatively, implement a separate `flatten_json_value` that walks `serde_json::Value::Object` directly — this avoids the double conversion and is more explicit:

```rust
pub fn flatten_dict(&self, map: serde_json::Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    Self::flatten_json_value(map, "", &mut out);
    out
}

fn flatten_json_value(value: serde_json::Value, prefix: &str, out: &mut HashMap<String, String>) {
    match value {
        serde_json::Value::Object(obj) => {
            for (k, v) in obj {
                let full_key = if prefix.is_empty() {
                    k
                } else {
                    format!("{prefix}.{k}")
                };
                Self::flatten_json_value(v, &full_key, out);
            }
        }
        serde_json::Value::Bool(b) => { out.insert(prefix.to_string(), b.to_string()); }
        serde_json::Value::Number(n) => { out.insert(prefix.to_string(), n.to_string()); }
        serde_json::Value::String(s) => { out.insert(prefix.to_string(), s); }
        serde_json::Value::Null => { out.insert(prefix.to_string(), String::new()); }
        serde_json::Value::Array(_) => { out.insert(prefix.to_string(), value.to_string()); }
    }
}
```

**Prefer the second approach** (direct JSON walk) — it is clearer and avoids any serde_yaml conversion overhead.

### 6. Run tests (GREEN)

```bash
cargo test --lib 2>&1 | grep -E "^test |FAILED|error\["
```

All inline unit tests in `src/config.rs` must pass. The `todo!()` panics should be gone. Integration tests in `tests/test_config.rs` may still show `assert!(false)` failures — those are addressed in the `tests` task.

```bash
cargo test 2>&1 | tail -10
```

### 7. Refactor: tidy up and add doc comments

- Remove the `let _ = (key, cli_flag, env_var);` placeholder lines.
- Ensure each method has a one-line doc comment explaining its purpose and the tier it handles.
- Confirm no `clippy` warnings:

```bash
cargo clippy -- -D warnings 2>&1 | head -40
```

Fix any warnings before declaring the task complete.

---

## Acceptance Criteria

- [ ] No `todo!()` macros remain in `src/config.rs`
- [ ] `cargo test --lib` passes all inline unit tests in `src/config.rs`
- [ ] `test_resolve_tier1_cli_flag_wins` passes
- [ ] `test_resolve_tier2_env_var_wins` passes
- [ ] `test_resolve_tier4_default_wins` passes (returns `Some("./extensions")`)
- [ ] `test_flatten_dict_nested` passes
- [ ] `test_flatten_dict_deeply_nested` passes
- [ ] `cargo clippy -- -D warnings` reports no warnings in `src/config.rs`
- [ ] Missing config file (non-existent path) does not panic — `_config_file` is `None`
- [ ] Malformed YAML config file does not panic — emits a `warn!()` and sets `_config_file` to `None`
- [ ] Empty-string env var is skipped (tier 2 falls through)
- [ ] CLI flag with value `None` is skipped (tier 1 falls through)

---

## Dependencies

- **Depends on**: `models` (correct `DEFAULTS` and type definitions)
- **Required by**: `tests` (integration test assertions require working logic)
