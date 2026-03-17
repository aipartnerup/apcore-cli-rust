// apcore-cli — Integration tests for output formatting.
// Protocol spec: FE-04

mod common;

use apcore_cli::output::{format_exec_result, format_module_detail, format_module_list, resolve_format};
use serde_json::json;

#[test]
fn test_resolve_format_explicit_json() {
    let fmt = resolve_format(Some("json"));
    assert_eq!(fmt, "json");
}

#[test]
fn test_resolve_format_explicit_table() {
    let fmt = resolve_format(Some("table"));
    assert_eq!(fmt, "table");
}

#[test]
fn test_resolve_format_none_defaults() {
    // Without explicit format, must return "table" or "json" based on TTY.
    // TODO: implement once resolve_format is complete.
    assert!(false, "not implemented");
}

#[test]
fn test_format_module_list_json_valid() {
    let modules = vec![
        json!({"module_id": "math.add", "description": "Add two numbers", "tags": []}),
        json!({"module_id": "text.upper", "description": "Uppercase", "tags": []}),
    ];
    let output = format_module_list(&modules, "json");
    // TODO: assert valid JSON array with 2 elements.
    assert!(false, "not implemented");
}

#[test]
fn test_format_module_list_table_has_headers() {
    let modules = vec![
        json!({"module_id": "math.add", "description": "Add two numbers", "tags": []}),
    ];
    let output = format_module_list(&modules, "table");
    // TODO: assert table output contains "ID" or "MODULE" header.
    assert!(false, "not implemented");
}

#[test]
fn test_format_module_detail_json() {
    let module = json!({
        "module_id": "math.add",
        "description": "Add two numbers",
        "input_schema": {},
        "tags": []
    });
    let output = format_module_detail(&module, "json");
    // TODO: assert output is valid JSON and contains module_id.
    assert!(false, "not implemented");
}

#[test]
fn test_format_exec_result_json() {
    let result = json!({"sum": 42});
    let output = format_exec_result(&result, "json");
    // TODO: assert output is valid JSON containing "sum".
    assert!(false, "not implemented");
}

#[test]
fn test_format_exec_result_table() {
    let result = json!({"sum": 42});
    let output = format_exec_result(&result, "table");
    // TODO: assert table output contains "sum" and "42".
    assert!(false, "not implemented");
}
