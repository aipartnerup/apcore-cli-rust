// apcore-cli — TTY-adaptive output formatting.
// Protocol spec: FE-04 (format_module_list, format_module_detail,
//                        format_exec_result, resolve_format)

use serde_json::Value;

// ---------------------------------------------------------------------------
// resolve_format
// ---------------------------------------------------------------------------

/// Determine the output format to use.
///
/// Resolution order:
/// 1. `explicit_format` if provided
/// 2. `"json"` when stdout is not a TTY
/// 3. `"table"` otherwise
pub fn resolve_format(explicit_format: Option<&str>) -> &'static str {
    // TODO: check explicit_format, then isatty(stdout), default to "table".
    let _ = explicit_format;
    todo!("resolve_format")
}

// ---------------------------------------------------------------------------
// format_module_list
// ---------------------------------------------------------------------------

/// Render a list of module descriptors as a table or JSON.
///
/// # Arguments
/// * `modules` — slice of `serde_json::Value` objects (module descriptors)
/// * `format`  — `"table"` or `"json"`
///
/// Returns the formatted string ready for printing to stdout.
pub fn format_module_list(modules: &[Value], format: &str) -> String {
    // TODO: table → comfy-table with columns [ID, DESCRIPTION, TAGS]
    //       json  → serde_json::to_string_pretty
    let _ = (modules, format);
    todo!("format_module_list")
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

    #[test]
    fn test_resolve_format_explicit_json() {
        // TODO: explicit "json" should return "json".
        assert!(false, "not implemented");
    }

    #[test]
    fn test_resolve_format_explicit_table() {
        // TODO: explicit "table" should return "table".
        assert!(false, "not implemented");
    }

    #[test]
    fn test_format_module_list_json() {
        // TODO: verify JSON output is valid and contains module ids.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_format_module_list_table() {
        // TODO: verify table output contains column headers.
        assert!(false, "not implemented");
    }

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
