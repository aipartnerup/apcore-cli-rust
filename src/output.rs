// apcore-cli — TTY-adaptive output formatting.
// Protocol spec: FE-04 (format_module_list, format_module_detail,
//                        format_exec_result, resolve_format)

use serde_json::Value;
use std::io::IsTerminal;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub(crate) const DESCRIPTION_TRUNCATE_LEN: usize = 80;

// ---------------------------------------------------------------------------
// resolve_format
// ---------------------------------------------------------------------------

/// Private inner: accepts explicit TTY state for testability.
pub(crate) fn resolve_format_inner(explicit_format: Option<&str>, is_tty: bool) -> &'static str {
    if let Some(fmt) = explicit_format {
        return match fmt {
            "json" => "json",
            "table" => "table",
            other => {
                // Unknown format: log a warning and fall back to json.
                // (Invalid values are caught by clap upstream; this is a safety net.)
                tracing::warn!("Unknown format '{}', defaulting to 'json'.", other);
                "json"
            }
        };
    }
    if is_tty { "table" } else { "json" }
}

/// Determine the output format to use.
///
/// Resolution order:
/// 1. `explicit_format` if `Some`.
/// 2. `"table"` when stdout is a TTY.
/// 3. `"json"` otherwise.
pub fn resolve_format(explicit_format: Option<&str>) -> &'static str {
    let is_tty = std::io::stdout().is_terminal();
    resolve_format_inner(explicit_format, is_tty)
}

// ---------------------------------------------------------------------------
// truncate
// ---------------------------------------------------------------------------

/// Truncate `text` to at most `max_length` characters.
///
/// If truncation occurs, the last 3 characters are replaced with `"..."`.
/// Uses char-boundary-safe truncation to handle Unicode correctly: byte length
/// is used for the boundary check (matching Python's `len()` on ASCII-dominant
/// module descriptions), but slicing respects char boundaries.
pub(crate) fn truncate(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        return text.to_string();
    }
    let cutoff = max_length.saturating_sub(3);
    // Walk back from cutoff to find a valid char boundary.
    let mut end = cutoff;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &text[..end])
}

// ---------------------------------------------------------------------------
// format_module_list helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// format_module_list
// ---------------------------------------------------------------------------

/// Render a list of module descriptors as a table or JSON.
///
/// # Arguments
/// * `modules`      — slice of `serde_json::Value` objects (module descriptors)
/// * `format`       — `"table"` or `"json"`
/// * `filter_tags`  — AND-filter: only modules that have ALL listed tags are shown
///
/// Returns the formatted string ready for printing to stdout.
pub fn format_module_list(modules: &[Value], format: &str, filter_tags: &[&str]) -> String {
    use comfy_table::{ContentArrangement, Table};

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
                let desc = truncate(desc_raw, DESCRIPTION_TRUNCATE_LEN);
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
            tracing::warn!(
                "Unknown format '{}' in format_module_list, using json.",
                unknown
            );
            format_module_list(modules, "json", filter_tags)
        }
    }
}

// ---------------------------------------------------------------------------
// format_module_detail
// ---------------------------------------------------------------------------

/// Render a single module descriptor with its full schema.
///
/// # Arguments
/// * `module` — `serde_json::Value` module descriptor
/// * `format` — `"table"` or `"json"`
pub fn format_module_detail(module: &Value, format: &str) -> String {
    // TODO: table → multi-section comfy-table (metadata + schema fields)
    //       json  → serde_json::to_string_pretty
    let _ = (module, format);
    todo!("format_module_detail")
}

// ---------------------------------------------------------------------------
// format_exec_result
// ---------------------------------------------------------------------------

/// Render a module execution result.
///
/// # Arguments
/// * `result` — `serde_json::Value` (the `output` field from the executor response)
/// * `format` — `"table"` or `"json"`
pub fn format_exec_result(result: &Value, format: &str) -> String {
    // TODO: table → key-value comfy-table
    //       json  → serde_json::to_string_pretty
    let _ = (result, format);
    todo!("format_exec_result")
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- resolve_format_inner ---

    #[test]
    fn test_resolve_format_explicit_json_tty() {
        // Explicit format wins over TTY state.
        assert_eq!(resolve_format_inner(Some("json"), true), "json");
    }

    #[test]
    fn test_resolve_format_explicit_table_non_tty() {
        // Explicit format wins over non-TTY state.
        assert_eq!(resolve_format_inner(Some("table"), false), "table");
    }

    #[test]
    fn test_resolve_format_none_tty() {
        // No explicit format + TTY → "table".
        assert_eq!(resolve_format_inner(None, true), "table");
    }

    #[test]
    fn test_resolve_format_none_non_tty() {
        // No explicit format + non-TTY → "json".
        assert_eq!(resolve_format_inner(None, false), "json");
    }

    // --- truncate ---

    #[test]
    fn test_truncate_short_string() {
        let s = "hello";
        assert_eq!(truncate(s, 80), "hello");
    }

    #[test]
    fn test_truncate_exact_length() {
        let s = "a".repeat(80);
        assert_eq!(truncate(&s, 80), s);
    }

    #[test]
    fn test_truncate_over_limit() {
        let s = "a".repeat(100);
        let result = truncate(&s, 80);
        assert_eq!(result.len(), 80);
        assert!(result.ends_with("..."));
        assert_eq!(&result[..77], &"a".repeat(77));
    }

    #[test]
    fn test_truncate_exactly_81_chars() {
        let s = "b".repeat(81);
        let result = truncate(&s, 80);
        assert_eq!(result.len(), 80);
        assert!(result.ends_with("..."));
    }

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
        assert!(output.contains("..."), "long description must be truncated with '...'");
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

    // Placeholder tests for future tasks (kept to avoid removing stubs needed by other tasks)
    #[test]
    fn test_format_module_detail_json() {
        // TODO: verify detail JSON output structure.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_format_exec_result_json() {
        // TODO: verify execution result JSON round-trips correctly.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_format_exec_result_table() {
        // TODO: verify table output contains result key-value pairs.
        assert!(false, "not implemented");
    }
}
