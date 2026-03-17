# Task: ref-resolver-core

**Feature**: FE-02 Schema Parser
**File**: `src/ref_resolver.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~3h
**Depends on**: none
**Required by**: `schema-composition`, `type-mapping`

---

## Context

`resolve_refs` must deep-copy the incoming schema, walk the tree, substitute every `{"$ref": "#/$defs/Key"}` node with the fully-resolved definition from `$defs`, enforce a depth limit of 32, and detect circular chains. The existing stub in `src/ref_resolver.rs` declares the correct public signature and `RefResolverError` type — the task is to fill in the bodies of `resolve_refs` and the private `resolve_node` helper.

The function must:

1. Deep-copy `schema` (do not mutate the caller's value; the `&mut Value` parameter is accepted but left unchanged per the design decision in `plan.md`).
2. Extract `defs` from `$defs` or `definitions` (whichever is present; `$defs` takes precedence).
3. Recursively resolve the copy via `resolve_node`.
4. Strip `$defs` and `definitions` from the returned value.

`resolve_node` handles only `$ref` substitution in this task. Composition keywords (`allOf`, `anyOf`, `oneOf`) are handled in the subsequent `schema-composition` task; `resolve_node` must skip them for now (leave as-is).

**Exit code semantics** (enforced by the caller in `cli.rs`, not by `resolve_refs` itself):

| Error variant | Meaning | Exit code |
|---------------|---------|-----------|
| `RefResolverError::Unresolvable` | `$ref` key not found in `$defs` | 45 |
| `RefResolverError::Circular` | same `$ref` path seen twice in one resolution chain | 48 |
| `RefResolverError::MaxDepthExceeded` | `depth >= max_depth` before resolving | 48 |

`resolve_refs` returns `Err(...)` and does not call `std::process::exit` itself. The caller maps errors to exit codes.

---

## RED — Write Failing Tests First

Replace the `assert!(false, "not implemented")` bodies in `tests/test_ref_resolver.rs` and add new cases. All tests must fail (function panics with `todo!`) before GREEN.

```rust
// tests/test_ref_resolver.rs

#[test]
fn test_resolve_refs_no_refs_returns_unchanged() {
    let mut schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        }
    });
    let result = resolve_refs(&mut schema, 32, "test.module");
    assert!(result.is_ok());
    let resolved = result.unwrap();
    assert_eq!(resolved["properties"]["name"]["type"], "string");
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
    let result = resolve_refs(&mut schema, 32, "test.module");
    assert!(result.is_ok());
    let resolved = result.unwrap();
    // $ref must be replaced by the definition content.
    assert_eq!(resolved["properties"]["name"]["type"], "string");
    assert_eq!(resolved["properties"]["name"]["description"], "A name");
    // $defs must be stripped from result.
    assert!(resolved.get("$defs").is_none());
}

#[test]
fn test_resolve_refs_definitions_key_also_supported() {
    // Some schemas use "definitions" instead of "$defs".
    let mut schema = json!({
        "definitions": {
            "Addr": {"type": "string"}
        },
        "properties": {
            "city": {"$ref": "#/definitions/Addr"}
        }
    });
    let result = resolve_refs(&mut schema, 32, "test.module");
    assert!(result.is_ok());
    let resolved = result.unwrap();
    assert_eq!(resolved["properties"]["city"]["type"], "string");
    assert!(resolved.get("definitions").is_none());
}

#[test]
fn test_resolve_refs_unresolvable_returns_error() {
    let mut schema = json!({
        "type": "object",
        "properties": {
            "x": {"$ref": "#/$defs/DoesNotExist"}
        }
    });
    let result = resolve_refs(&mut schema, 32, "test.module");
    assert!(
        matches!(result, Err(RefResolverError::Unresolvable { .. })),
        "expected Unresolvable, got: {result:?}"
    );
}

#[test]
fn test_resolve_refs_circular_returns_error() {
    let mut schema = json!({
        "$defs": {
            "A": {"$ref": "#/$defs/B"},
            "B": {"$ref": "#/$defs/A"}
        },
        "properties": {
            "x": {"$ref": "#/$defs/A"}
        }
    });
    let result = resolve_refs(&mut schema, 32, "test.module");
    assert!(
        matches!(
            result,
            Err(RefResolverError::Circular { .. }) | Err(RefResolverError::MaxDepthExceeded { .. })
        ),
        "expected Circular or MaxDepthExceeded, got: {result:?}"
    );
}

#[test]
fn test_resolve_refs_max_depth_exceeded() {
    // max_depth=0 means the first $ref hit immediately fails.
    let mut schema = json!({
        "$defs": {
            "Inner": {"type": "string"}
        },
        "properties": {
            "x": {"$ref": "#/$defs/Inner"}
        }
    });
    let result = resolve_refs(&mut schema, 0, "test.module");
    assert!(
        matches!(result, Err(RefResolverError::MaxDepthExceeded { .. })),
        "expected MaxDepthExceeded, got: {result:?}"
    );
}

#[test]
fn test_resolve_refs_nested_properties() {
    // $refs inside nested object properties must also be resolved.
    let mut schema = json!({
        "$defs": {
            "City": {"type": "string"}
        },
        "properties": {
            "address": {
                "type": "object",
                "properties": {
                    "city": {"$ref": "#/$defs/City"}
                }
            }
        }
    });
    let result = resolve_refs(&mut schema, 32, "test.module");
    assert!(result.is_ok());
    let resolved = result.unwrap();
    assert_eq!(
        resolved["properties"]["address"]["properties"]["city"]["type"],
        "string"
    );
}

#[test]
fn test_resolve_refs_does_not_mutate_input() {
    // The original schema must not be modified.
    let original = json!({
        "$defs": {"T": {"type": "integer"}},
        "properties": {"x": {"$ref": "#/$defs/T"}}
    });
    let mut schema = original.clone();
    let _ = resolve_refs(&mut schema, 32, "test.module");
    // Input schema still has $ref (not mutated).
    assert_eq!(schema["properties"]["x"]["$ref"], "#/$defs/T");
}
```

Run `cargo test test_resolve_refs` — all should fail (panics at `todo!`).

---

## GREEN — Implement

```rust
// src/ref_resolver.rs

use std::collections::HashSet;
use serde_json::{Value, Map};

pub fn resolve_refs(
    schema: &mut Value,
    max_depth: usize,
    module_id: &str,
) -> Result<Value, RefResolverError> {
    // Deep-copy; do not modify the caller's value.
    let mut copy = schema.clone();

    // Extract $defs / definitions.
    let defs: Map<String, Value> = {
        let raw = copy
            .get("$defs")
            .or_else(|| copy.get("definitions"))
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        raw
    };

    let resolved = resolve_node(copy, &defs, &mut HashSet::new(), 0, max_depth, module_id)?;

    // Strip definition keys from result.
    let mut result = resolved;
    if let Some(obj) = result.as_object_mut() {
        obj.remove("$defs");
        obj.remove("definitions");
    }
    Ok(result)
}

fn resolve_node(
    node: Value,
    defs: &Map<String, Value>,
    visited: &mut HashSet<String>,
    depth: usize,
    max_depth: usize,
    module_id: &str,
) -> Result<Value, RefResolverError> {
    let obj = match node {
        Value::Object(map) => map,
        other => return Ok(other),
    };

    // Handle $ref.
    if let Some(ref_val) = obj.get("$ref") {
        let ref_path = ref_val
            .as_str()
            .unwrap_or("")
            .to_string();

        if depth >= max_depth {
            return Err(RefResolverError::MaxDepthExceeded {
                max_depth,
                module_id: module_id.to_string(),
            });
        }

        if visited.contains(&ref_path) {
            return Err(RefResolverError::Circular {
                module_id: module_id.to_string(),
            });
        }

        // Extract key: "#/$defs/Address" → "Address"
        let key = ref_path
            .split('/')
            .last()
            .unwrap_or("")
            .to_string();

        let def = defs.get(&key).cloned().ok_or_else(|| {
            RefResolverError::Unresolvable {
                reference: ref_path.clone(),
                module_id: module_id.to_string(),
            }
        })?;

        visited.insert(ref_path);
        let result = resolve_node(def, defs, visited, depth + 1, max_depth, module_id)?;
        // Remove the inserted ref_path so sibling refs can reuse the same path.
        // (visited is used to detect cycles within a single resolution chain, not globally)
        // Do NOT remove here — keep it for the duration of this chain.
        return Ok(result);
    }

    // Recursively resolve all values in the object map (handles nested properties).
    let mut resolved_map = Map::with_capacity(obj.len());
    for (k, v) in obj {
        let resolved_v = resolve_node(v, defs, visited, depth, max_depth, module_id)?;
        resolved_map.insert(k, resolved_v);
    }

    Ok(Value::Object(resolved_map))
}
```

Key design notes:

- `visited` is a `HashSet<String>` passed by `&mut` so siblings do not block each other. Add `ref_path` before recursing and the set remains populated for the full chain, preventing A→B→A circular paths.
- The recursive call on all map values (not just `"properties"`) covers `allOf`/`anyOf`/`oneOf` sub-schemas, nested `properties`, and arbitrary extension keys that may contain `$ref`.
- `schema` (the `&mut Value` parameter) is never mutated; `schema.clone()` produces the working copy on line one.

---

## REFACTOR

- Confirm `serde_json::Map` is imported as `serde_json::Map` (it re-exports `IndexMap` or `BTreeMap` depending on features; the concrete type does not matter here).
- Run `cargo clippy -- -D warnings`; address any `unused_import` or shadowing warnings.
- Ensure the `visited` removal strategy (keep-in-chain, do not remove after recursion) correctly handles the case where the same `$ref` appears in two different properties of the same object — they must not block each other. This is achieved because `visited` tracks the chain depth, not global occurrences. Write a test for this case if not already covered.

---

## Verification

```bash
cargo test test_resolve_refs 2>&1
# Expected: test result: ok. 8 passed; 0 failed
```
