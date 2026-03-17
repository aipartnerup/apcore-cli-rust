// apcore-cli — Integration tests for JSON Schema $ref inliner.
// Protocol spec: FE-08

mod common;

use apcore_cli::ref_resolver::{resolve_refs, RefResolverError};
use serde_json::json;

#[test]
fn test_resolve_refs_no_refs_returns_unchanged() {
    let mut schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        }
    });
    let result = resolve_refs(&mut schema, 10, "test.module");
    // TODO: assert result is Ok and schema is unchanged.
    assert!(false, "not implemented");
}

#[test]
fn test_resolve_refs_simple_inline() {
    let mut schema = json!({
        "$defs": {
            "MyString": {"type": "string", "description": "A name"}
        },
        "type": "object",
        "properties": {
            "name": {"$ref": "#/$defs/MyString"}
        }
    });
    let result = resolve_refs(&mut schema, 10, "test.module");
    // TODO: assert name property is inlined with type="string".
    assert!(false, "not implemented");
}

#[test]
fn test_resolve_refs_unresolvable_returns_error() {
    let mut schema = json!({
        "type": "object",
        "properties": {
            "x": {"$ref": "#/$defs/DoesNotExist"}
        }
    });
    let result = resolve_refs(&mut schema, 10, "test.module");
    assert!(matches!(result, Err(RefResolverError::Unresolvable { .. })));
}

#[test]
fn test_resolve_refs_circular_returns_error() {
    // A → B → A must produce a Circular error.
    let mut schema = json!({
        "$defs": {
            "A": {"$ref": "#/$defs/B"},
            "B": {"$ref": "#/$defs/A"}
        },
        "type": "object",
        "properties": {
            "x": {"$ref": "#/$defs/A"}
        }
    });
    let result = resolve_refs(&mut schema, 20, "test.module");
    // TODO: assert Circular or MaxDepthExceeded error.
    assert!(false, "not implemented");
}

#[test]
fn test_resolve_refs_max_depth_exceeded() {
    // max_depth=1 on a 2-level schema must return MaxDepthExceeded.
    let mut schema = json!({
        "$defs": {
            "Inner": {"type": "string"}
        },
        "type": "object",
        "properties": {
            "x": {"$ref": "#/$defs/Inner"}
        }
    });
    let result = resolve_refs(&mut schema, 0, "test.module");
    assert!(matches!(
        result,
        Err(RefResolverError::MaxDepthExceeded { .. })
    ));
}

#[test]
fn test_resolve_refs_nested_properties() {
    // $refs inside nested properties must all be resolved.
    assert!(false, "not implemented");
}
