# Task: reconvert-enum-values

**Feature**: FE-02 Schema Parser
**File**: `src/schema_parser.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~1h
**Depends on**: `enum-choices`
**Required by**: `update-clap-arg-call-site`

---

## Context

After clap parses CLI arguments, all values arrive as `String` (clap stores everything as `OsString`, which is then read as `String` by the dispatcher). For enum properties whose original JSON values were `Number` or `Bool`, the dispatcher must convert the string back to the correct `serde_json::Value` type before passing the merged input to the module executor.

The existing stub `reconvert_enum_values(kwargs, args)` takes `&[clap::Arg]` as the second argument. This task replaces it with `reconvert_enum_values(kwargs, schema_args)` taking `&SchemaArgs`, using `schema_args.enum_maps` for the lookup.

The function does not modify `bool_pairs`; boolean values are reconciled separately in `collect_input` (tracked in `update-clap-arg-call-site`).

---

## Conversion Rules

| Original JSON type | How to detect | Conversion |
|--------------------|---------------|------------|
| `Value::Number` (integer) | `original.is_i64()` or `original.is_u64()` | `str.parse::<i64>()` → `Value::Number` |
| `Value::Number` (float) | `original.is_f64()` | `str.parse::<f64>()` → `Value::Number` via `serde_json::Number::from_f64` |
| `Value::Bool` | `original.is_bool()` | `str == "true"` → `Value::Bool(true)`, else `Value::Bool(false)` |
| `Value::String` | otherwise | keep as-is |

The lookup strategy: given a property name `key` and its string value `str_val`, find `schema_args.enum_maps.get(key)`, then find the original `Value` in that vec whose string representation matches `str_val`. Use the original value's type to decide the conversion.

---

## RED — Write Failing Tests First

Replace the existing `assert!(false, "not implemented")` stubs in `tests/test_schema_parser.rs` and add new cases:

```rust
use apcore_cli::schema_parser::{reconvert_enum_values, schema_to_clap_args, SchemaArgs};
use serde_json::{json, Value};
use std::collections::HashMap;

fn make_kwargs(pairs: &[(&str, &str)]) -> HashMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
        .collect()
}

#[test]
fn test_reconvert_string_enum_passthrough() {
    let schema = json!({
        "properties": {"format": {"type": "string", "enum": ["json", "csv"]}}
    });
    let schema_args = schema_to_clap_args(&schema).unwrap();
    let kwargs = make_kwargs(&[("format", "json")]);
    let result = reconvert_enum_values(kwargs, &schema_args);
    assert_eq!(result["format"], Value::String("json".to_string()));
}

#[test]
fn test_reconvert_integer_enum() {
    let schema = json!({
        "properties": {"level": {"type": "integer", "enum": [1, 2, 3]}}
    });
    let schema_args = schema_to_clap_args(&schema).unwrap();
    let kwargs = make_kwargs(&[("level", "2")]);
    let result = reconvert_enum_values(kwargs, &schema_args);
    assert_eq!(result["level"], json!(2));
    // Must be a JSON number, not a string.
    assert!(result["level"].is_number());
}

#[test]
fn test_reconvert_float_enum() {
    let schema = json!({
        "properties": {"ratio": {"type": "number", "enum": [0.5, 1.0, 1.5]}}
    });
    let schema_args = schema_to_clap_args(&schema).unwrap();
    let kwargs = make_kwargs(&[("ratio", "1.5")]);
    let result = reconvert_enum_values(kwargs, &schema_args);
    assert!(result["ratio"].is_number());
    assert_eq!(result["ratio"].as_f64(), Some(1.5));
}

#[test]
fn test_reconvert_bool_enum() {
    // Non-boolean type with bool enum values.
    let schema = json!({
        "properties": {"strict": {"type": "string", "enum": [true, false]}}
    });
    let schema_args = schema_to_clap_args(&schema).unwrap();
    let kwargs = make_kwargs(&[("strict", "true")]);
    let result = reconvert_enum_values(kwargs, &schema_args);
    assert_eq!(result["strict"], Value::Bool(true));
}

#[test]
fn test_reconvert_non_enum_field_unchanged() {
    let schema = json!({
        "properties": {"name": {"type": "string"}}
    });
    let schema_args = schema_to_clap_args(&schema).unwrap();
    let kwargs = make_kwargs(&[("name", "alice")]);
    let result = reconvert_enum_values(kwargs, &schema_args);
    assert_eq!(result["name"], Value::String("alice".to_string()));
}

#[test]
fn test_reconvert_null_value_unchanged() {
    let schema = json!({
        "properties": {"mode": {"type": "string", "enum": ["a", "b"]}}
    });
    let schema_args = schema_to_clap_args(&schema).unwrap();
    let mut kwargs: HashMap<String, Value> = HashMap::new();
    kwargs.insert("mode".to_string(), Value::Null);
    let result = reconvert_enum_values(kwargs, &schema_args);
    // Null values (absent optional arg) must pass through unchanged.
    assert_eq!(result["mode"], Value::Null);
}

#[test]
fn test_reconvert_preserves_non_enum_keys() {
    // Other keys in kwargs must be returned unchanged even if not in schema.
    let schema = json!({
        "properties": {"format": {"type": "string", "enum": ["json"]}}
    });
    let schema_args = schema_to_clap_args(&schema).unwrap();
    let mut kwargs = make_kwargs(&[("format", "json")]);
    kwargs.insert("extra".to_string(), Value::String("untouched".to_string()));
    let result = reconvert_enum_values(kwargs, &schema_args);
    assert_eq!(result["extra"], Value::String("untouched".to_string()));
}
```

Run `cargo test test_reconvert` — all fail.

---

## GREEN — Implement

Replace the stub in `src/schema_parser.rs`:

```rust
pub fn reconvert_enum_values(
    kwargs: HashMap<String, Value>,
    schema_args: &SchemaArgs,
) -> HashMap<String, Value> {
    let mut result = kwargs;

    for (key, original_variants) in &schema_args.enum_maps {
        let val = match result.get(key) {
            Some(v) => v.clone(),
            None => continue,
        };

        // Skip null / non-string values (absent optional args arrive as Null).
        let str_val = match &val {
            Value::String(s) => s.clone(),
            _ => continue,
        };

        // Find the original variant whose string form matches str_val.
        let original = original_variants.iter().find(|v| {
            let as_str = match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            as_str == str_val
        });

        if let Some(orig) = original {
            let converted = match orig {
                Value::Number(_) => {
                    if orig.is_i64() || orig.is_u64() {
                        str_val
                            .parse::<i64>()
                            .ok()
                            .map(|n| Value::Number(n.into()))
                            .unwrap_or(val.clone())
                    } else {
                        str_val
                            .parse::<f64>()
                            .ok()
                            .and_then(serde_json::Number::from_f64)
                            .map(Value::Number)
                            .unwrap_or(val.clone())
                    }
                }
                Value::Bool(_) => Value::Bool(str_val.to_lowercase() == "true"),
                _ => val.clone(), // String: keep as-is
            };
            result.insert(key.clone(), converted);
        }
    }

    result
}
```

Note: `Value::Number` does not implement `is_i64()` directly; use `orig.as_i64().is_some()` as the check:

```rust
Value::Number(n) => {
    if n.as_i64().is_some() {
        str_val
            .parse::<i64>()
            .ok()
            .map(|i| Value::Number(i.into()))
            .unwrap_or(val.clone())
    } else {
        str_val
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(val.clone())
    }
}
```

---

## REFACTOR

- Update `lib.rs` to export `reconvert_enum_values` with the new signature. The existing `pub use schema_parser::reconvert_enum_values;` line compiles only if the signature matches; this is the compile gate that ensures the call site in `cli.rs` is updated.
- Run `cargo clippy -- -D warnings`.

---

## Verification

```bash
cargo test test_reconvert 2>&1
# Expected: test result: ok. 7 passed; 0 failed
```
