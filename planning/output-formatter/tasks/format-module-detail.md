# Task: format-module-detail

**Feature**: FE-08 Output Formatter
**File**: `src/output.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~2h
**Depends on**: `resolve-format-and-truncate`
**Required by**: `wire-format-flag`

---

## Context

`format_module_detail` renders a single module descriptor's full metadata as either a multi-section plain-text display (the `"table"` format) or a filtered JSON object.

The Python implementation uses `rich.panel.Panel` for the header, `rich.syntax.Syntax` for JSON-highlighted schemas, and prints each section with `click.echo`. The Rust version:

- Replaces `Panel` with a `print_panel` helper that wraps a `comfy_table::Table` configured with a box border.
- Replaces `rich.syntax.Syntax` with plain `serde_json::to_string_pretty` (no color).
- Returns `String` (all sections concatenated) instead of printing directly.

The module descriptor `Value` object fields read by this function:

| Field | JSON key | Condition |
|-------|----------|-----------|
| Module ID | `"module_id"` / `"id"` / `"canonical_id"` | Always present |
| Description | `"description"` | Always present |
| Input schema | `"input_schema"` | Printed if present and non-null |
| Output schema | `"output_schema"` | Printed if present and non-null |
| Annotations | `"annotations"` | Printed if present, non-null, and non-empty object |
| Extension metadata | keys starting with `"x-"` or `"x_"` | Printed if any exist |
| Tags | `"tags"` | Printed if non-empty array |

---

## RED — Write Failing Tests First

Add to the `#[cfg(test)]` block in `src/output.rs`:

```rust
    // --- format_module_detail ---

    #[test]
    fn test_format_module_detail_json_full() {
        let module = json!({
            "module_id": "math.add",
            "description": "Add two numbers",
            "input_schema": {"type": "object", "properties": {"a": {"type": "integer"}}},
            "output_schema": {"type": "object", "properties": {"result": {"type": "integer"}}},
            "tags": ["math"],
            "annotations": {"author": "test"}
        });
        let output = format_module_detail(&module, "json");
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("must be valid JSON");
        assert_eq!(parsed["id"], "math.add");
        assert_eq!(parsed["description"], "Add two numbers");
        assert!(parsed.get("input_schema").is_some(), "input_schema must be present");
        assert!(parsed.get("output_schema").is_some(), "output_schema must be present");
        let tags = parsed["tags"].as_array().unwrap();
        assert_eq!(tags[0], "math");
    }

    #[test]
    fn test_format_module_detail_json_no_output_schema() {
        let module = json!({
            "module_id": "text.upper",
            "description": "Uppercase",
        });
        let output = format_module_detail(&module, "json");
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.get("output_schema").is_none(), "output_schema must be absent when not set");
    }

    #[test]
    fn test_format_module_detail_json_no_none_fields() {
        let module = json!({
            "module_id": "a.b",
            "description": "desc",
            "input_schema": null,
            "output_schema": null,
            "tags": null,
        });
        let output = format_module_detail(&module, "json");
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.get("input_schema").is_none(), "null input_schema must be absent");
        assert!(parsed.get("tags").is_none(), "null tags must be absent");
    }

    #[test]
    fn test_format_module_detail_table_contains_description() {
        let module = json!({
            "module_id": "math.add",
            "description": "Add two numbers",
        });
        let output = format_module_detail(&module, "table");
        assert!(output.contains("Add two numbers"), "table must contain description");
    }

    #[test]
    fn test_format_module_detail_table_contains_module_id() {
        let module = json!({
            "module_id": "math.add",
            "description": "desc",
        });
        let output = format_module_detail(&module, "table");
        assert!(output.contains("math.add"), "table must contain module ID");
    }

    #[test]
    fn test_format_module_detail_table_input_schema_section() {
        let module = json!({
            "module_id": "math.add",
            "description": "desc",
            "input_schema": {"type": "object"}
        });
        let output = format_module_detail(&module, "table");
        assert!(output.contains("Input Schema"), "table must contain Input Schema section");
    }

    #[test]
    fn test_format_module_detail_table_no_output_schema_section_when_absent() {
        let module = json!({
            "module_id": "text.upper",
            "description": "desc",
        });
        let output = format_module_detail(&module, "table");
        assert!(
            !output.contains("Output Schema"),
            "Output Schema section must be absent when not set"
        );
    }

    #[test]
    fn test_format_module_detail_table_tags_section() {
        let module = json!({
            "module_id": "math.add",
            "description": "desc",
            "tags": ["math", "arithmetic"]
        });
        let output = format_module_detail(&module, "table");
        assert!(output.contains("Tags"), "table must contain Tags section");
        assert!(output.contains("math"), "table must contain tag value");
    }

    #[test]
    fn test_format_module_detail_table_annotations_section() {
        let module = json!({
            "module_id": "a.b",
            "description": "desc",
            "annotations": {"author": "alice", "version": "1.0"}
        });
        let output = format_module_detail(&module, "table");
        assert!(output.contains("Annotations"), "table must contain Annotations section");
        assert!(output.contains("author"), "table must contain annotation key");
        assert!(output.contains("alice"), "table must contain annotation value");
    }

    #[test]
    fn test_format_module_detail_table_extension_metadata() {
        let module = json!({
            "module_id": "a.b",
            "description": "desc",
            "x-category": "utility"
        });
        let output = format_module_detail(&module, "table");
        assert!(output.contains("Extension Metadata"), "must contain Extension Metadata section");
        assert!(output.contains("x-category"), "must contain x- key");
        assert!(output.contains("utility"), "must contain x- value");
    }
```

Run `cargo test test_format_module_detail` — all fail (stub `todo!` panics).

---

## GREEN — Implement

Replace the `format_module_detail` stub in `src/output.rs`:

```rust
/// Render a minimal bordered panel heading. Returns a String with a box around `title`.
fn render_panel(title: &str) -> String {
    let mut table = Table::new();
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.add_row(vec![title]);
    table.to_string()
}

pub fn format_module_detail(module: &Value, format: &str) -> String {
    let id = extract_str(module, &["module_id", "id", "canonical_id"]);
    let description = extract_str(module, &["description"]);

    match format {
        "table" => {
            let mut parts: Vec<String> = Vec::new();

            // Header panel.
            parts.push(render_panel(&format!("Module: {}", id)));

            // Description.
            parts.push(format!("\nDescription:\n  {}\n", description));

            // Input schema.
            if let Some(input_schema) = module.get("input_schema").filter(|v| !v.is_null()) {
                parts.push("\nInput Schema:".to_string());
                parts.push(
                    serde_json::to_string_pretty(input_schema)
                        .unwrap_or_else(|_| "{}".to_string()),
                );
            }

            // Output schema.
            if let Some(output_schema) = module.get("output_schema").filter(|v| !v.is_null()) {
                parts.push("\nOutput Schema:".to_string());
                parts.push(
                    serde_json::to_string_pretty(output_schema)
                        .unwrap_or_else(|_| "{}".to_string()),
                );
            }

            // Annotations.
            if let Some(ann) = module.get("annotations").and_then(|v| v.as_object()) {
                if !ann.is_empty() {
                    parts.push("\nAnnotations:".to_string());
                    for (k, v) in ann {
                        parts.push(format!("  {}: {}", k, v));
                    }
                }
            }

            // Extension metadata (x- prefixed keys at the top level).
            let x_fields: Vec<(&str, &Value)> = module
                .as_object()
                .map(|obj| {
                    obj.iter()
                        .filter(|(k, _)| k.starts_with("x-") || k.starts_with("x_"))
                        .map(|(k, v)| (k.as_str(), v))
                        .collect()
                })
                .unwrap_or_default();
            if !x_fields.is_empty() {
                parts.push("\nExtension Metadata:".to_string());
                for (k, v) in x_fields {
                    parts.push(format!("  {}: {}", k, v));
                }
            }

            // Tags.
            let tags = extract_tags(module);
            if !tags.is_empty() {
                parts.push(format!("\nTags: {}", tags.join(", ")));
            }

            parts.join("\n")
        }
        "json" => {
            let mut result = serde_json::Map::new();
            result.insert("id".to_string(), serde_json::Value::String(id.to_string()));
            result.insert(
                "description".to_string(),
                serde_json::Value::String(description.to_string()),
            );

            // Optional fields: only include if present and non-null.
            for key in &["input_schema", "output_schema"] {
                if let Some(v) = module.get(*key).filter(|v| !v.is_null()) {
                    result.insert(key.to_string(), v.clone());
                }
            }

            if let Some(ann) = module.get("annotations").filter(|v| !v.is_null() && v.as_object().map_or(false, |o| !o.is_empty())) {
                result.insert("annotations".to_string(), ann.clone());
            }

            let tags = extract_tags(module);
            if !tags.is_empty() {
                result.insert(
                    "tags".to_string(),
                    serde_json::Value::Array(
                        tags.into_iter().map(serde_json::Value::String).collect(),
                    ),
                );
            }

            // Extension metadata.
            if let Some(obj) = module.as_object() {
                for (k, v) in obj {
                    if k.starts_with("x-") || k.starts_with("x_") {
                        result.insert(k.clone(), v.clone());
                    }
                }
            }

            serde_json::to_string_pretty(&serde_json::Value::Object(result))
                .unwrap_or_else(|_| "{}".to_string())
        }
        unknown => {
            tracing::warn!("Unknown format '{}' in format_module_detail, using json.", unknown);
            format_module_detail(module, "json")
        }
    }
}
```

**Notes:**
- `comfy_table::presets::UTF8_FULL` gives the panel a full Unicode box border. If the terminal does not support Unicode, fall back to `comfy_table::presets::ASCII_FULL` by detecting `NO_COLOR` or `TERM=dumb` in a future iteration.
- `serde_json::Value` display (via `{}`) uses its `Display` implementation which renders unquoted scalars and JSON-quoted strings. For annotation values you may want to strip quotes: use `v.as_str().unwrap_or(&v.to_string())` to print strings without surrounding quotes.

---

## REFACTOR

- Extract a `render_optional_section(label: &str, value: &Value) -> Option<String>` helper to DRY up input/output schema rendering.
- Confirm `comfy_table::presets::UTF8_FULL` is available in `comfy-table = "7"`.
- Run `cargo clippy -- -D warnings`.

---

## Verification

```bash
cargo test test_format_module_detail 2>&1
# Expected: 10 tests pass, 0 fail.
```
