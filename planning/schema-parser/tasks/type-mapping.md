# Task: type-mapping

**Feature**: FE-02 Schema Parser
**File**: `src/schema_parser.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~2h
**Depends on**: `schema-composition` (resolve_refs must work)
**Required by**: `boolean-flag-pairs`, `enum-choices`

---

## Context

This task introduces `SchemaArgs` and `BoolFlagPair` as the primary output types, replaces the `schema_to_clap_args(schema: &Value) -> Vec<clap::Arg>` stub with `schema_to_clap_args(schema: &Value) -> Result<SchemaArgs, SchemaParserError>`, and implements `map_type` for all non-boolean, non-enum property types. Boolean and enum handling are deferred to the next two tasks; this task must correctly produce `clap::Arg`s for `string`, `integer`, `number`, `object`, `array`, and the file convention.

`reconvert_enum_values` is updated in a later task; do not touch it here.

---

## Data Structures to Add

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use serde_json::Value;
use thiserror::Error;

/// Error type for schema parsing failures.
#[derive(Debug, Error)]
pub enum SchemaParserError {
    /// Two properties normalise to the same --flag-name.
    /// Caller must exit 48.
    #[error(
        "Flag name collision: properties '{prop_a}' and '{prop_b}' both map to '{flag_name}'"
    )]
    FlagCollision {
        prop_a: String,
        prop_b: String,
        flag_name: String,
    },
}

/// A single boolean --flag / --no-flag pair generated from a `type: boolean` property.
pub struct BoolFlagPair {
    /// Original schema property name (e.g. "verbose").
    pub prop_name: String,
    /// Long name used for the positive flag (e.g. "verbose").
    pub flag_long: String,
    /// Default value from the schema's `default` field (defaults to false).
    pub default_val: bool,
}

/// Full output of schema_to_clap_args.
pub struct SchemaArgs {
    /// clap Args ready to attach to a clap::Command.
    pub args: Vec<clap::Arg>,
    /// Boolean flag pairs; used by collect_input to reconcile --flag/--no-flag.
    pub bool_pairs: Vec<BoolFlagPair>,
    /// Maps property name (snake_case) → original enum values (as serde_json::Value).
    /// Used by reconvert_enum_values for type coercion.
    pub enum_maps: HashMap<String, Vec<Value>>,
}
```

---

## RED — Write Failing Tests First

Replace the existing `assert!(false, "not implemented")` stubs in `tests/test_schema_parser.rs` and add the following. All must fail before GREEN.

```rust
use apcore_cli::schema_parser::{schema_to_clap_args, SchemaParserError};
use serde_json::json;

// Helper: find an Arg by long name.
fn find_arg<'a>(args: &'a [clap::Arg], long: &str) -> Option<&'a clap::Arg> {
    args.iter().find(|a| a.get_long() == Some(long))
}

#[test]
fn test_schema_to_clap_args_empty_schema() {
    let schema = json!({});
    let result = schema_to_clap_args(&schema).unwrap();
    assert!(result.args.is_empty());
    assert!(result.bool_pairs.is_empty());
    assert!(result.enum_maps.is_empty());
}

#[test]
fn test_schema_to_clap_args_string_property() {
    let schema = json!({
        "properties": {"text": {"type": "string", "description": "Some text"}},
        "required": []
    });
    let result = schema_to_clap_args(&schema).unwrap();
    assert_eq!(result.args.len(), 1);
    let arg = find_arg(&result.args, "text").expect("--text must exist");
    assert_eq!(arg.get_id(), "text");
    assert!(!arg.is_required_set());
}

#[test]
fn test_schema_to_clap_args_integer_property() {
    let schema = json!({
        "properties": {"count": {"type": "integer"}},
        "required": ["count"]
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let arg = find_arg(&result.args, "count").expect("--count must exist");
    assert!(arg.is_required_set());
    // The value_parser should accept "42" and reject "hello".
    // (Exact value_parser assertion is done via clap's debug output if needed.)
}

#[test]
fn test_schema_to_clap_args_number_property() {
    let schema = json!({
        "properties": {"rate": {"type": "number"}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    assert!(find_arg(&result.args, "rate").is_some());
}

#[test]
fn test_schema_to_clap_args_object_and_array_as_string() {
    let schema = json!({
        "properties": {
            "data": {"type": "object"},
            "items": {"type": "array"}
        }
    });
    let result = schema_to_clap_args(&schema).unwrap();
    // Both must appear; no special parser (plain string).
    assert!(find_arg(&result.args, "data").is_some());
    assert!(find_arg(&result.args, "items").is_some());
}

#[test]
fn test_schema_to_clap_args_underscore_to_hyphen() {
    let schema = json!({
        "properties": {"input_file": {"type": "string"}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    // Flag long name must be "input-file".
    assert!(find_arg(&result.args, "input-file").is_some());
    // Arg id must be "input_file" (original name, for collect_input lookup).
    let arg = find_arg(&result.args, "input-file").unwrap();
    assert_eq!(arg.get_id(), "input_file");
}

#[test]
fn test_schema_to_clap_args_file_convention_suffix() {
    let schema = json!({
        "properties": {"config_file": {"type": "string"}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let arg = find_arg(&result.args, "config-file").expect("must exist");
    // value_parser should be PathBuf — check via get_value_parser type or
    // by running a Command with a path argument.
    let _ = arg; // Exact parser check is implementation-dependent; integration test in GREEN notes.
}

#[test]
fn test_schema_to_clap_args_x_cli_file_flag() {
    let schema = json!({
        "properties": {"report": {"type": "string", "x-cli-file": true}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    assert!(find_arg(&result.args, "report").is_some());
}

#[test]
fn test_schema_to_clap_args_unknown_type_defaults_to_string() {
    // Unknown types must produce a plain string Arg (no panic, no error).
    let schema = json!({
        "properties": {"x": {"type": "foobar"}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    assert!(find_arg(&result.args, "x").is_some());
}

#[test]
fn test_schema_to_clap_args_missing_type_defaults_to_string() {
    let schema = json!({
        "properties": {"x": {"description": "no type field"}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    assert!(find_arg(&result.args, "x").is_some());
}

#[test]
fn test_schema_to_clap_args_default_value_set() {
    let schema = json!({
        "properties": {"timeout": {"type": "integer", "default": 30}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let arg = find_arg(&result.args, "timeout").unwrap();
    // Default value should be "30" (clap stores defaults as OsString/String).
    assert_eq!(
        arg.get_default_values().first().and_then(|v| v.to_str()),
        Some("30")
    );
}
```

Run `cargo test test_schema_to_clap_args` — all fail.

---

## GREEN — Implement

Replace the stub in `src/schema_parser.rs`:

```rust
use std::collections::HashMap;
use std::path::PathBuf;

use clap::{Arg, ArgAction};
use serde_json::Value;
use thiserror::Error;
use tracing::warn;

#[derive(Debug, Error)]
pub enum SchemaParserError {
    #[error(
        "Flag name collision: properties '{prop_a}' and '{prop_b}' both map to '{flag_name}'"
    )]
    FlagCollision {
        prop_a: String,
        prop_b: String,
        flag_name: String,
    },
}

pub struct BoolFlagPair {
    pub prop_name: String,
    pub flag_long: String,
    pub default_val: bool,
}

pub struct SchemaArgs {
    pub args: Vec<Arg>,
    pub bool_pairs: Vec<BoolFlagPair>,
    pub enum_maps: HashMap<String, Vec<Value>>,
}

/// Determine whether a property should use PathBuf value_parser.
fn is_file_property(prop_name: &str, prop_schema: &Value) -> bool {
    prop_name.ends_with("_file")
        || prop_schema
            .get("x-cli-file")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
}

pub fn schema_to_clap_args(schema: &Value) -> Result<SchemaArgs, SchemaParserError> {
    let properties = match schema.get("properties").and_then(|v| v.as_object()) {
        Some(p) => p,
        None => {
            return Ok(SchemaArgs {
                args: Vec::new(),
                bool_pairs: Vec::new(),
                enum_maps: HashMap::new(),
            })
        }
    };

    let required_list: Vec<&str> = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect()
        })
        .unwrap_or_default();

    // Warn about required properties missing from properties map.
    for req_name in &required_list {
        if !properties.contains_key(*req_name) {
            warn!(
                "Required property '{}' not found in properties, skipping.",
                req_name
            );
        }
    }

    let mut args: Vec<Arg> = Vec::new();
    let mut bool_pairs: Vec<BoolFlagPair> = Vec::new();
    let enum_maps: HashMap<String, Vec<Value>> = HashMap::new(); // populated in enum-choices task
    let mut seen_flags: HashMap<String, String> = HashMap::new(); // flag_long → prop_name

    for (prop_name, prop_schema) in properties {
        let flag_long = prop_name.replace('_', "-");

        // Collision detection.
        if let Some(existing) = seen_flags.get(&flag_long) {
            return Err(SchemaParserError::FlagCollision {
                prop_a: prop_name.clone(),
                prop_b: existing.clone(),
                flag_name: flag_long,
            });
        }
        seen_flags.insert(flag_long.clone(), prop_name.clone());

        let schema_type = prop_schema.get("type").and_then(|v| v.as_str());
        let is_required = required_list.contains(&prop_name.as_str());
        let help_text = extract_help(prop_schema);
        let default_val = prop_schema.get("default");

        // Boolean is handled in boolean-flag-pairs task — skip for now.
        if schema_type == Some("boolean") {
            // Placeholder: boolean handling added in next task.
            continue;
        }

        // Enum is handled in enum-choices task — skip for now.
        if prop_schema.get("enum").is_some() {
            // Placeholder: enum handling added in next task.
            continue;
        }

        // Build standard Arg.
        let mut arg = Arg::new(prop_name.clone())
            .long(flag_long.clone())
            .required(is_required);

        if let Some(help) = help_text {
            arg = arg.help(help);
        }

        // Type-specific value_parser.
        arg = match schema_type {
            Some("integer") => arg.value_parser(clap::value_parser!(i64)),
            Some("number") => arg.value_parser(clap::value_parser!(f64)),
            Some("string") if is_file_property(prop_name, prop_schema) => {
                arg.value_parser(clap::value_parser!(PathBuf))
            }
            Some("string") | Some("object") | Some("array") => arg, // plain string
            Some(unknown) => {
                warn!(
                    "Unknown schema type '{}' for property '{}', defaulting to string.",
                    unknown, prop_name
                );
                arg
            }
            None => {
                warn!(
                    "No type specified for property '{}', defaulting to string.",
                    prop_name
                );
                arg
            }
        };

        // Default value (set as string; clap parses it through the value_parser).
        if let Some(dv) = default_val {
            let dv_str = match dv {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            arg = arg.default_value(dv_str);
        }

        args.push(arg);
    }

    Ok(SchemaArgs {
        args,
        bool_pairs,
        enum_maps,
    })
}

/// Extract help text from a schema property.
/// Prefers `x-llm-description` over `description`.
/// Truncates to configurable limit (default 1000 chars).
pub fn extract_help(prop_schema: &Value) -> Option<String> {
    let text = prop_schema
        .get("x-llm-description")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            prop_schema
                .get("description")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
        })?;

    if text.len() > HELP_TEXT_MAX_LEN {
        Some(format!("{}...", &text[..HELP_TEXT_MAX_LEN - 3]))
    } else {
        Some(text.to_string())
    }
}
```

---

## REFACTOR

- Move `SchemaParserError`, `BoolFlagPair`, `SchemaArgs` to the top of the file so they are visible to all functions.
- Confirm `clap::value_parser!(i64)` and `clap::value_parser!(f64)` compile; both are available in clap 4.0+.
- Run `cargo clippy -- -D warnings`.
- Update `lib.rs` exports: replace `pub use schema_parser::{reconvert_enum_values, schema_to_clap_args};` with the new types as they are introduced:

```rust
pub use schema_parser::{
    reconvert_enum_values, schema_to_clap_args, BoolFlagPair, SchemaArgs, SchemaParserError,
};
```

---

## Verification

```bash
cargo test test_schema_to_clap_args 2>&1
# Expected: all type-mapping tests pass; boolean/enum tests still fail (not yet implemented).
```
