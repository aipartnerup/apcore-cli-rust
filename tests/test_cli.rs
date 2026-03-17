// apcore-cli — Integration tests for CLI dispatcher.
// Protocol spec: FE-01 (build_module_command, collect_input, validate_module_id)

mod common;

use std::collections::HashMap;

use apcore_cli::cli::{build_module_command, collect_input, validate_module_id};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// validate_module_id
// ---------------------------------------------------------------------------

#[test]
fn test_validate_module_id_valid_ids() {
    for id in ["math.add", "text.summarize", "a", "a.b.c"] {
        assert!(
            validate_module_id(id).is_ok(),
            "expected ok for '{id}'"
        );
    }
}

#[test]
fn test_validate_module_id_too_long() {
    let long_id = "a".repeat(129);
    assert!(validate_module_id(&long_id).is_err());
}

#[test]
fn test_validate_module_id_invalid_formats() {
    for id in ["INVALID!ID", "123abc", ".leading.dot", "a..b", "a."] {
        assert!(
            validate_module_id(id).is_err(),
            "expected error for '{id}'"
        );
    }
}

#[test]
fn test_validate_module_id_max_length_ok() {
    let max_id = "a".repeat(128);
    assert!(validate_module_id(&max_id).is_ok());
}

// ---------------------------------------------------------------------------
// collect_input
// ---------------------------------------------------------------------------

#[test]
fn test_collect_input_no_stdin_drops_null_values() {
    // None values in cli_kwargs must be dropped.
    let mut kwargs = HashMap::new();
    kwargs.insert("a".to_string(), json!(5));
    kwargs.insert("b".to_string(), Value::Null);
    let result = collect_input(None, kwargs, false);
    // TODO: assert result == {"a": 5}
    assert!(false, "not implemented");
}

#[test]
fn test_collect_input_stdin_valid_json() {
    // TODO: inject stdin with valid JSON object, assert merged result.
    assert!(false, "not implemented");
}

#[test]
fn test_collect_input_cli_overrides_stdin() {
    // CLI flags must override STDIN values for the same key.
    // TODO: inject stdin with {"a": 5}, cli_kwargs={"a": 99}, assert a==99.
    assert!(false, "not implemented");
}

#[test]
fn test_collect_input_oversized_stdin_rejected() {
    // Input exceeding 10 MiB with large_input=false must return an error.
    // TODO: mock stdin with >10 MiB payload, assert InputTooLarge error.
    assert!(false, "not implemented");
}

#[test]
fn test_collect_input_large_input_allowed() {
    // large_input=true must accept payloads >10 MiB.
    assert!(false, "not implemented");
}

#[test]
fn test_collect_input_invalid_json_returns_error() {
    // Non-JSON stdin must return CliError::JsonParse.
    assert!(false, "not implemented");
}

#[test]
fn test_collect_input_non_object_json_returns_error() {
    // JSON arrays / scalars must return CliError::NotAnObject.
    assert!(false, "not implemented");
}

#[test]
fn test_collect_input_empty_stdin_returns_empty_map() {
    // Empty stdin must produce an empty HashMap.
    assert!(false, "not implemented");
}

// ---------------------------------------------------------------------------
// build_module_command
// ---------------------------------------------------------------------------

#[test]
fn test_build_module_command_creates_command() {
    // build_module_command must return a Command named after the module.
    // TODO: construct a mock module descriptor, call build_module_command,
    //       verify name and about.
    assert!(false, "not implemented");
}
