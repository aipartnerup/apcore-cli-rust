// apcore-cli tests — shared helpers and fixtures
// Used by all integration test modules via `mod common;`

use std::collections::HashMap;

use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Mock module descriptor
// ---------------------------------------------------------------------------

/// Build a minimal module descriptor Value for use in tests.
///
/// # Arguments
/// * `module_id`   — e.g. `"math.add"`
/// * `description` — short description string
pub fn sample_module_descriptor(module_id: &str, description: &str) -> Value {
    json!({
        "module_id": module_id,
        "description": description,
        "input_schema": {
            "type": "object",
            "properties": {},
            "required": []
        },
        "output_schema": {
            "type": "object",
            "properties": {}
        },
        "tags": [],
        "annotations": null
    })
}

/// Build a module descriptor with explicit input schema properties.
pub fn sample_module_with_schema(
    module_id: &str,
    description: &str,
    properties: Value,
    required: Vec<&str>,
) -> Value {
    json!({
        "module_id": module_id,
        "description": description,
        "input_schema": {
            "type": "object",
            "properties": properties,
            "required": required
        },
        "output_schema": {
            "type": "object",
            "properties": {}
        },
        "tags": [],
        "annotations": null
    })
}

// ---------------------------------------------------------------------------
// Mock execution result
// ---------------------------------------------------------------------------

/// Build a sample executor response wrapping `output`.
pub fn sample_exec_result(output: Value) -> Value {
    json!({
        "module_id": "math.add",
        "output": output,
        "error": null,
        "duration_ms": 1
    })
}

// ---------------------------------------------------------------------------
// Environment helpers
// ---------------------------------------------------------------------------

/// Remove all `APCORE_` environment variables from the current process.
///
/// Call this at the start of tests that need a clean environment.
/// Note: in Rust integration tests, prefer setting env vars with
/// `std::env::set_var` inside the test and restoring them afterwards.
pub fn strip_apcore_env_vars() {
    for (key, _) in std::env::vars() {
        if key.starts_with("APCORE_") {
            // SAFETY: tests run sequentially within a process; acceptable.
            unsafe { std::env::remove_var(&key) };
        }
    }
}
