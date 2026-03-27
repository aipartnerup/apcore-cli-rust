// apcore-cli -- Display overlay helpers for grouped commands.
// Protocol spec: FE-09 (display overlay resolution)

use serde_json::Value;

/// Extract the resolved display overlay from a module's metadata.
pub fn get_display(descriptor: &Value) -> Value {
    descriptor
        .get("metadata")
        .and_then(|m| m.get("display"))
        .cloned()
        .unwrap_or(Value::Null)
}

/// Return (display_name, description, tags) resolved from the
/// display overlay with CLI-specific fallback chain.
pub fn get_cli_display_fields(descriptor: &Value) -> (String, String, Vec<String>) {
    let display = get_display(descriptor);
    let cli = display.get("cli").unwrap_or(&Value::Null);

    let name = cli
        .get("alias")
        .and_then(|v| v.as_str())
        .or_else(|| display.get("alias").and_then(|v| v.as_str()))
        .or_else(|| descriptor.get("id").and_then(|v| v.as_str()))
        .or_else(|| descriptor.get("module_id").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

    let desc = cli
        .get("description")
        .and_then(|v| v.as_str())
        .or_else(|| descriptor.get("description").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

    let tags = display
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .or_else(|| {
            descriptor
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
        })
        .unwrap_or_default();

    (name, desc, tags)
}

// -------------------------------------------------------------------
// Unit tests
// -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_display_returns_display_from_metadata() {
        let descriptor = json!({
            "metadata": {
                "display": {
                    "alias": "greet",
                    "tags": ["demo"]
                }
            }
        });
        let display = get_display(&descriptor);
        assert_eq!(display["alias"], "greet");
    }

    #[test]
    fn test_get_display_returns_null_when_missing() {
        let descriptor = json!({"module_id": "a.b"});
        assert!(get_display(&descriptor).is_null());
    }

    #[test]
    fn test_get_display_returns_null_when_no_display_key() {
        let descriptor = json!({"metadata": {"version": "1.0"}});
        assert!(get_display(&descriptor).is_null());
    }

    #[test]
    fn test_cli_display_fields_cli_alias_wins() {
        let descriptor = json!({
            "module_id": "math.add",
            "metadata": {
                "display": {
                    "alias": "top-alias",
                    "cli": { "alias": "cli-alias" }
                }
            }
        });
        let (name, _, _) = get_cli_display_fields(&descriptor);
        assert_eq!(name, "cli-alias");
    }

    #[test]
    fn test_cli_display_fields_display_alias_fallback() {
        let descriptor = json!({
            "module_id": "math.add",
            "metadata": {
                "display": { "alias": "top-alias" }
            }
        });
        let (name, _, _) = get_cli_display_fields(&descriptor);
        assert_eq!(name, "top-alias");
    }

    #[test]
    fn test_cli_display_fields_id_fallback() {
        let descriptor = json!({"id": "my-id"});
        let (name, _, _) = get_cli_display_fields(&descriptor);
        assert_eq!(name, "my-id");
    }

    #[test]
    fn test_cli_display_fields_module_id_fallback() {
        let descriptor = json!({"module_id": "math.add"});
        let (name, _, _) = get_cli_display_fields(&descriptor);
        assert_eq!(name, "math.add");
    }

    #[test]
    fn test_cli_display_fields_empty_when_no_name() {
        let descriptor = json!({});
        let (name, _, _) = get_cli_display_fields(&descriptor);
        assert_eq!(name, "");
    }

    #[test]
    fn test_cli_display_fields_description_from_cli() {
        let descriptor = json!({
            "description": "top-desc",
            "metadata": {
                "display": {
                    "cli": { "description": "cli-desc" }
                }
            }
        });
        let (_, desc, _) = get_cli_display_fields(&descriptor);
        assert_eq!(desc, "cli-desc");
    }

    #[test]
    fn test_cli_display_fields_description_fallback() {
        let descriptor = json!({"description": "top-desc"});
        let (_, desc, _) = get_cli_display_fields(&descriptor);
        assert_eq!(desc, "top-desc");
    }

    #[test]
    fn test_cli_display_fields_tags_from_display() {
        let descriptor = json!({
            "tags": ["a"],
            "metadata": {
                "display": { "tags": ["b", "c"] }
            }
        });
        let (_, _, tags) = get_cli_display_fields(&descriptor);
        assert_eq!(tags, vec!["b", "c"]);
    }

    #[test]
    fn test_cli_display_fields_tags_fallback_to_descriptor() {
        let descriptor = json!({"tags": ["x", "y"]});
        let (_, _, tags) = get_cli_display_fields(&descriptor);
        assert_eq!(tags, vec!["x", "y"]);
    }

    #[test]
    fn test_cli_display_fields_tags_empty_default() {
        let descriptor = json!({});
        let (_, _, tags) = get_cli_display_fields(&descriptor);
        assert!(tags.is_empty());
    }
}
