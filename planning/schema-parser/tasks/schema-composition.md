# Task: schema-composition

**Feature**: FE-02 Schema Parser
**File**: `src/ref_resolver.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~2h
**Depends on**: `ref-resolver-core`
**Required by**: `type-mapping`

---

## Context

Extend `resolve_node` (implemented in `ref-resolver-core`) to handle the three JSON Schema composition keywords: `allOf`, `anyOf`, and `oneOf`. These mirror the Python `_resolve_node` composition branches.

Rules:

| Keyword | Properties | `required` |
|---------|-----------|-----------|
| `allOf` | Union of all sub-schemas; later sub-schema wins on key conflict | Concatenation (extend) |
| `anyOf` | Union of all sub-schemas | Intersection (only fields required in ALL branches) |
| `oneOf` | Same as `anyOf` | Intersection |

Non-composition keys from the parent node (e.g., `"description"`, `"title"`) are carried into the merged result if not already set by the merge.

This task adds branches to `resolve_node` before the generic map-recursion fallthrough that was added in `ref-resolver-core`.

---

## RED — Write Failing Tests First

Add to `tests/test_ref_resolver.rs`:

```rust
#[test]
fn test_allof_merges_properties() {
    let mut schema = json!({
        "allOf": [
            {
                "properties": {"a": {"type": "string"}},
                "required": ["a"]
            },
            {
                "properties": {"b": {"type": "integer"}},
                "required": ["b"]
            }
        ]
    });
    let result = resolve_refs(&mut schema, 32, "mod").unwrap();
    assert_eq!(result["properties"]["a"]["type"], "string");
    assert_eq!(result["properties"]["b"]["type"], "integer");
    let required: Vec<&str> = result["required"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(required.contains(&"a"));
    assert!(required.contains(&"b"));
}

#[test]
fn test_allof_later_schema_wins_on_conflict() {
    let mut schema = json!({
        "allOf": [
            {"properties": {"x": {"type": "string"}}},
            {"properties": {"x": {"type": "integer"}}}
        ]
    });
    let result = resolve_refs(&mut schema, 32, "mod").unwrap();
    // Later sub-schema wins: x must be integer.
    assert_eq!(result["properties"]["x"]["type"], "integer");
}

#[test]
fn test_allof_copies_non_composition_keys() {
    let mut schema = json!({
        "description": "My type",
        "allOf": [
            {"properties": {"a": {"type": "string"}}}
        ]
    });
    let result = resolve_refs(&mut schema, 32, "mod").unwrap();
    // "description" must survive in the merged result.
    assert_eq!(result["description"], "My type");
}

#[test]
fn test_anyof_unions_properties() {
    let mut schema = json!({
        "anyOf": [
            {"properties": {"a": {"type": "string"}}, "required": ["a"]},
            {"properties": {"b": {"type": "integer"}}, "required": ["b"]}
        ]
    });
    let result = resolve_refs(&mut schema, 32, "mod").unwrap();
    // Both properties must appear.
    assert!(result["properties"].get("a").is_some());
    assert!(result["properties"].get("b").is_some());
}

#[test]
fn test_anyof_required_is_intersection() {
    let mut schema = json!({
        "anyOf": [
            {"properties": {"a": {"type": "string"}, "b": {"type": "string"}}, "required": ["a", "b"]},
            {"properties": {"a": {"type": "string"}, "c": {"type": "string"}}, "required": ["a", "c"]}
        ]
    });
    let result = resolve_refs(&mut schema, 32, "mod").unwrap();
    let required: Vec<&str> = result["required"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    // Only "a" appears in both branches — it is the intersection.
    assert!(required.contains(&"a"), "a must be required (in both branches)");
    assert!(!required.contains(&"b"), "b must not be required (only in first branch)");
    assert!(!required.contains(&"c"), "c must not be required (only in second branch)");
}

#[test]
fn test_anyof_empty_required_when_no_overlap() {
    let mut schema = json!({
        "anyOf": [
            {"properties": {"a": {"type": "string"}}, "required": ["a"]},
            {"properties": {"b": {"type": "integer"}}, "required": ["b"]}
        ]
    });
    let result = resolve_refs(&mut schema, 32, "mod").unwrap();
    let required = result["required"].as_array().unwrap();
    assert!(required.is_empty(), "no fields are required in both branches");
}

#[test]
fn test_oneof_behaves_like_anyof() {
    let mut schema = json!({
        "oneOf": [
            {"properties": {"x": {"type": "string"}}, "required": ["x"]},
            {"properties": {"y": {"type": "integer"}}, "required": ["y"]}
        ]
    });
    let result = resolve_refs(&mut schema, 32, "mod").unwrap();
    assert!(result["properties"].get("x").is_some());
    assert!(result["properties"].get("y").is_some());
    assert!(result["required"].as_array().unwrap().is_empty());
}

#[test]
fn test_allof_with_nested_ref() {
    // allOf sub-schema that itself contains a $ref.
    let mut schema = json!({
        "$defs": {
            "Base": {"properties": {"id": {"type": "integer"}}, "required": ["id"]}
        },
        "allOf": [
            {"$ref": "#/$defs/Base"},
            {"properties": {"name": {"type": "string"}}}
        ]
    });
    let result = resolve_refs(&mut schema, 32, "mod").unwrap();
    assert_eq!(result["properties"]["id"]["type"], "integer");
    assert_eq!(result["properties"]["name"]["type"], "string");
    let required: Vec<&str> = result["required"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(required.contains(&"id"));
}
```

Run `cargo test test_allof test_anyof test_oneof` — all fail.

---

## GREEN — Implement

Add composition branches to `resolve_node` in `src/ref_resolver.rs`, inserted before the generic map-recursion fallthrough:

```rust
// Inside resolve_node, after the $ref branch and before the generic fallthrough:

// Handle allOf.
if obj.contains_key("allOf") {
    let sub_schemas = obj
        .get("allOf")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut merged_props = Map::new();
    let mut merged_required: Vec<Value> = Vec::new();

    for sub in sub_schemas {
        let resolved_sub =
            resolve_node(sub, defs, visited, depth + 1, max_depth, module_id)?;
        if let Some(props) = resolved_sub.get("properties").and_then(|v| v.as_object()) {
            for (k, v) in props {
                merged_props.insert(k.clone(), v.clone());
            }
        }
        if let Some(req) = resolved_sub.get("required").and_then(|v| v.as_array()) {
            merged_required.extend(req.iter().cloned());
        }
    }

    let mut result_map = Map::new();
    result_map.insert("properties".to_string(), Value::Object(merged_props));
    result_map.insert("required".to_string(), Value::Array(merged_required));

    // Carry over non-composition keys from the parent node.
    for (k, v) in &obj {
        if k != "allOf" && !result_map.contains_key(k) {
            result_map.insert(k.clone(), v.clone());
        }
    }

    return Ok(Value::Object(result_map));
}

// Handle anyOf / oneOf (same logic).
for keyword in &["anyOf", "oneOf"] {
    if obj.contains_key(*keyword) {
        let sub_schemas = obj
            .get(*keyword)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut merged_props = Map::new();
        let mut all_required_sets: Vec<std::collections::HashSet<String>> = Vec::new();

        for sub in sub_schemas {
            let resolved_sub =
                resolve_node(sub, defs, visited, depth + 1, max_depth, module_id)?;
            if let Some(props) = resolved_sub.get("properties").and_then(|v| v.as_object()) {
                for (k, v) in props {
                    merged_props.insert(k.clone(), v.clone());
                }
            }
            if let Some(req) = resolved_sub.get("required").and_then(|v| v.as_array()) {
                let set: std::collections::HashSet<String> = req
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect();
                all_required_sets.push(set);
            } else {
                // A branch with no required means the intersection is empty.
                all_required_sets.push(std::collections::HashSet::new());
            }
        }

        // Intersection of all required sets.
        let intersection: Vec<Value> = if all_required_sets.is_empty() {
            Vec::new()
        } else {
            let mut iter = all_required_sets.into_iter();
            let first = iter.next().unwrap();
            iter.fold(first, |acc, set| {
                acc.intersection(&set).cloned().collect()
            })
            .into_iter()
            .map(Value::String)
            .collect()
        };

        let mut result_map = Map::new();
        result_map.insert("properties".to_string(), Value::Object(merged_props));
        result_map.insert("required".to_string(), Value::Array(intersection));

        for (k, v) in &obj {
            if k != *keyword && !result_map.contains_key(k) {
                result_map.insert(k.clone(), v.clone());
            }
        }

        return Ok(Value::Object(result_map));
    }
}
```

---

## REFACTOR

- Factor out the `required`-intersection logic into a private `intersect_required_sets(sets: Vec<HashSet<String>>) -> Vec<Value>` function to avoid duplication between `anyOf` and `oneOf`.
- Confirm that `visited` is correctly threaded through sub-schema recursion for `allOf`/`anyOf`/`oneOf` so that circular refs within composition are still detected.
- Run `cargo clippy -- -D warnings`.

---

## Verification

```bash
cargo test test_allof test_anyof test_oneof 2>&1
# Expected: test result: ok. 8 passed; 0 failed
```
