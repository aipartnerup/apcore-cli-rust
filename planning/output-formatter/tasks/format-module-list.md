# Task: format-module-list

**Feature**: FE-08 Output Formatter
**File**: `src/output.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~2h
**Depends on**: `resolve-format-and-truncate`
**Required by**: `wire-format-flag`

---

## Context

`format_module_list` renders a slice of `serde_json::Value` module descriptors as either a `comfy-table` table or a JSON array. It is the Rust equivalent of the Python function of the same name.

Key differences from the Python implementation:

- Returns `String` instead of printing via `click.echo`. The caller (discovery command handler) prints the result to stdout. This makes the function testable without stdout capture.
- `filter_tags` is `&[&str]` instead of `tuple[str, ...]`.
- No `rich` formatting; uses `comfy_table::Table` with default styling.
- The existing stub signature `format_module_list(modules: &[Value], format: &str) -> String` is extended to `format_module_list(modules: &[Value], format: &str, filter_tags: &[&str]) -> String`.

The module descriptor `Value` objects are expected to have the following fields (all optional with graceful fallback):

| Field | JSON key | Fallback |
|-------|----------|---------|
| Module ID | `"module_id"` or `"id"` or `"canonical_id"` | `"<unknown>"` |
| Description | `"description"` | `""` |
| Tags | `"tags"` (JSON array of strings) | `[]` |

---

## RED — Write Failing Tests First

Add to the `#[cfg(test)]` block in `src/output.rs`:

```rust
    // --- format_module_list ---

    #[test]
    fn test_format_module_list_json_two_modules() {
        let modules = vec![
            json!({"module_id": "math.add", "description": "Add numbers", "tags": ["math"]}),
            json!({"module_id": "text.upper", "description": "Uppercase", "tags": []}),
        ];
        let output = format_module_list(&modules, "json", &[]);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("must be valid JSON");
        let arr = parsed.as_array().expect("must be array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["id"], "math.add");
        assert_eq!(arr[1]["id"], "text.upper");
    }

    #[test]
    fn test_format_module_list_json_empty() {
        let output = format_module_list(&[], "json", &[]);
        assert_eq!(output.trim(), "[]");
    }

    #[test]
    fn test_format_module_list_table_two_modules() {
        let modules = vec![
            json!({"module_id": "math.add", "description": "Add numbers", "tags": ["math"]}),
        ];
        let output = format_module_list(&modules, "table", &[]);
        assert!(output.contains("math.add"), "table must contain module ID");
        assert!(output.contains("Add numbers"), "table must contain description");
    }

    #[test]
    fn test_format_module_list_table_columns() {
        let modules = vec![
            json!({"module_id": "math.add", "description": "Add numbers", "tags": []}),
        ];
        let output = format_module_list(&modules, "table", &[]);
        // Column headers must be present.
        assert!(output.contains("ID"), "table must have ID column");
        assert!(output.contains("Description"), "table must have Description column");
        assert!(output.contains("Tags"), "table must have Tags column");
    }

    #[test]
    fn test_format_module_list_table_empty_no_tags() {
        let output = format_module_list(&[], "table", &[]);
        assert_eq!(output.trim(), "No modules found.");
    }

    #[test]
    fn test_format_module_list_table_empty_with_filter_tags() {
        let output = format_module_list(&[], "table", &["math", "text"]);
        assert!(
            output.contains("No modules found matching tags:"),
            "must contain tag-filter message"
        );
        assert!(output.contains("math"), "must contain tag name");
        assert!(output.contains("text"), "must contain tag name");
    }

    #[test]
    fn test_format_module_list_table_description_truncated() {
        let long_desc = "a".repeat(100);
        let modules = vec![
            json!({"module_id": "x.y", "description": long_desc, "tags": []}),
        ];
        let output = format_module_list(&modules, "table", &[]);
        // Truncated description: 77 chars + "..."
        assert!(output.contains("..."), "long description must be truncated with '...'");
        // Full 100-char description must NOT appear verbatim.
        assert!(!output.contains(&"a".repeat(100)), "full description must not appear");
    }

    #[test]
    fn test_format_module_list_json_tags_present() {
        let modules = vec![
            json!({"module_id": "a.b", "description": "desc", "tags": ["x", "y"]}),
        ];
        let output = format_module_list(&modules, "json", &[]);
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        let tags = parsed[0]["tags"].as_array().unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0], "x");
    }
```

Run `cargo test test_format_module_list` — all fail (stub `todo!` panics).

---

## GREEN — Implement

Replace the `format_module_list` stub in `src/output.rs`:

```rust
use comfy_table::{Table, ContentArrangement};

/// Extract a string field from a JSON module descriptor with fallback keys.
fn extract_str<'a>(v: &'a Value, keys: &[&str]) -> &'a str {
    for key in keys {
        if let Some(s) = v.get(key).and_then(|s| s.as_str()) {
            return s;
        }
    }
    ""
}

/// Extract tags array from a JSON module descriptor. Returns empty Vec on missing/invalid.
fn extract_tags(v: &Value) -> Vec<String> {
    v.get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

pub fn format_module_list(modules: &[Value], format: &str, filter_tags: &[&str]) -> String {
    match format {
        "table" => {
            if modules.is_empty() {
                if !filter_tags.is_empty() {
                    return format!(
                        "No modules found matching tags: {}.",
                        filter_tags.join(", ")
                    );
                }
                return "No modules found.".to_string();
            }

            let mut table = Table::new();
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec!["ID", "Description", "Tags"]);

            for m in modules {
                let id = extract_str(m, &["module_id", "id", "canonical_id"]);
                let desc_raw = extract_str(m, &["description"]);
                let desc = truncate(desc_raw, 80);
                let tags = extract_tags(m).join(", ");
                table.add_row(vec![id.to_string(), desc, tags]);
            }

            table.to_string()
        }
        "json" => {
            let result: Vec<serde_json::Value> = modules
                .iter()
                .map(|m| {
                    let id = extract_str(m, &["module_id", "id", "canonical_id"]);
                    let desc = extract_str(m, &["description"]);
                    let tags: Vec<serde_json::Value> = extract_tags(m)
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect();
                    serde_json::json!({
                        "id": id,
                        "description": desc,
                        "tags": tags,
                    })
                })
                .collect();

            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "[]".to_string())
        }
        unknown => {
            tracing::warn!("Unknown format '{}' in format_module_list, using json.", unknown);
            format_module_list(modules, "json", filter_tags)
        }
    }
}
```

**Notes:**
- `comfy_table::ContentArrangement::Dynamic` wraps column content to the terminal width. This is the idiomatic `comfy-table` approach for adaptive-width tables.
- `table.to_string()` returns the rendered table as a `String`; no stdout interaction.
- `extract_str` tries multiple key names (`module_id`, `id`, `canonical_id`) to handle both the apcore protocol format (`module_id`) and the normalised CLI cache format (`id` or `canonical_id`). This mirrors the Python `hasattr(m, "canonical_id")` fallback.

---

## REFACTOR

- Move `extract_str` and `extract_tags` to the top of the helper section (above `truncate`) to share across `format_module_list` and `format_module_detail`.
- Replace `table.to_string()` with explicit `format!("{}", table)` if clippy prefers it.
- Confirm `comfy_table` version 7 API: `Table::new()`, `set_header`, `add_row`, `to_string()` — all stable in v7.
- Run `cargo clippy -- -D warnings`.

---

## Verification

```bash
cargo test test_format_module_list 2>&1
# Expected: 8 tests pass, 0 fail.
```
