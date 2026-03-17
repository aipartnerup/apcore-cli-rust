// apcore-cli — JSON Schema → clap Arg translator.
// Protocol spec: FE-09 (schema_to_clap_args, reconvert_enum_values)

use std::collections::HashMap;

use serde_json::Value;

// ---------------------------------------------------------------------------
// schema_to_clap_args
// ---------------------------------------------------------------------------

/// Translate a JSON Schema `properties` map into a list of `clap::Arg`s.
///
/// Each schema property becomes one `--<name>` flag with:
/// * `help` set to the property's `description` field
/// * `required` set when the property appears in the schema's `required` array
/// * enum variants converted to clap `possible_values`
///
/// # Arguments
/// * `schema` — JSON Schema object (must have `"properties"` key)
///
/// Returns an empty Vec for schemas without properties.
pub fn schema_to_clap_args(schema: &Value) -> Vec<clap::Arg> {
    // TODO: iterate schema["properties"], build clap::Arg per field,
    //       mark required fields, attach possible_values for enum types.
    let _ = schema;
    todo!("schema_to_clap_args")
}

// ---------------------------------------------------------------------------
// reconvert_enum_values
// ---------------------------------------------------------------------------

/// Re-map string enum values from CLI args back to their JSON-typed forms.
///
/// clap always produces `String` values; this function converts them to the
/// correct JSON type (number, boolean, null) based on the original schema
/// definition.
///
/// # Arguments
/// * `kwargs` — raw CLI arguments map (string values from clap)
/// * `args`   — the clap `Arg` list produced by `schema_to_clap_args`
///
/// Returns a new map with values converted to their correct JSON types.
pub fn reconvert_enum_values(
    kwargs: HashMap<String, Value>,
    args: &[clap::Arg],
) -> HashMap<String, Value> {
    // TODO: for each kwarg, check the matching Arg's enum variants and
    //       coerce the string value to the correct JSON type.
    let _ = (kwargs, args);
    todo!("reconvert_enum_values")
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_schema_to_clap_args_empty_schema() {
        // Schema without properties must return an empty vec.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_schema_to_clap_args_string_field() {
        // A string property must produce a --name flag.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_schema_to_clap_args_required_field() {
        // A field in the `required` array must be marked required.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_schema_to_clap_args_enum_field() {
        // An enum field must have possible_values set.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_reconvert_enum_values_string_passthrough() {
        // A string enum value must be returned as-is.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_reconvert_enum_values_number_coercion() {
        // A numeric enum value supplied as string must become a JSON number.
        assert!(false, "not implemented");
    }
}
