# Task: enum-choices

**Feature**: FE-02 Schema Parser
**File**: `src/schema_parser.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~2h
**Depends on**: `type-mapping`
**Required by**: `help-text-and-collision`, `reconvert-enum-values`

---

## Context

Properties with an `"enum"` field (and `type` != `"boolean"`) must constrain the accepted values to the listed variants. Clap v4 enforces this via `clap::builder::PossibleValuesParser`. All enum values are converted to `String` for clap; the original typed values (int, float, bool, string) are stored in `SchemaArgs.enum_maps` for post-parse reconversion.

Empty enum arrays produce a warning log and fall through to a plain string `Arg` (matching Python behaviour).

---

## RED — Write Failing Tests First

Add to `tests/test_schema_parser.rs`:

```rust
#[test]
fn test_enum_string_choices() {
    let schema = json!({
        "properties": {
            "format": {"type": "string", "enum": ["json", "csv", "xml"]}
        }
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let arg = find_arg(&result.args, "format").expect("--format must exist");
    // Verify possible values via clap's possible_values() accessor.
    let possible: Vec<&str> = arg
        .get_possible_values()
        .iter()
        .map(|pv| pv.get_name())
        .collect();
    assert_eq!(possible, vec!["json", "csv", "xml"]);
}

#[test]
fn test_enum_integer_choices_as_strings() {
    let schema = json!({
        "properties": {
            "level": {"type": "integer", "enum": [1, 2, 3]}
        }
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let arg = find_arg(&result.args, "level").expect("--level must exist");
    let possible: Vec<&str> = arg
        .get_possible_values()
        .iter()
        .map(|pv| pv.get_name())
        .collect();
    assert_eq!(possible, vec!["1", "2", "3"]);
    // enum_maps must record the original Value types.
    let map = result.enum_maps.get("level").expect("enum_maps must have 'level'");
    assert_eq!(map[0], serde_json::Value::Number(1.into()));
}

#[test]
fn test_enum_float_choices_as_strings() {
    let schema = json!({
        "properties": {
            "ratio": {"type": "number", "enum": [0.5, 1.0, 1.5]}
        }
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let possible: Vec<&str> = find_arg(&result.args, "ratio")
        .unwrap()
        .get_possible_values()
        .iter()
        .map(|pv| pv.get_name())
        .collect();
    // JSON serialisation of floats: serde_json renders 0.5 as "0.5", 1.0 as "1.0".
    assert!(possible.contains(&"0.5"));
}

#[test]
fn test_enum_bool_choices_as_strings() {
    // Non-boolean type with bool enum values (unusual but valid per spec).
    let schema = json!({
        "properties": {
            "flag": {"type": "string", "enum": [true, false]}
        }
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let arg = find_arg(&result.args, "flag").expect("--flag must exist");
    let possible: Vec<&str> = arg
        .get_possible_values()
        .iter()
        .map(|pv| pv.get_name())
        .collect();
    assert!(possible.contains(&"true"));
    assert!(possible.contains(&"false"));
}

#[test]
fn test_enum_empty_array_falls_through_to_string() {
    // Empty enum → warning (not tested here) + plain string Arg.
    let schema = json!({
        "properties": {
            "x": {"type": "string", "enum": []}
        }
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let arg = find_arg(&result.args, "x").expect("--x must exist");
    // No possible_values should be set.
    assert!(arg.get_possible_values().is_empty());
    // Not in enum_maps.
    assert!(!result.enum_maps.contains_key("x"));
}

#[test]
fn test_enum_with_default() {
    let schema = json!({
        "properties": {
            "format": {"type": "string", "enum": ["json", "table"], "default": "json"}
        }
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let arg = find_arg(&result.args, "format").unwrap();
    assert_eq!(
        arg.get_default_values().first().and_then(|v| v.to_str()),
        Some("json")
    );
}

#[test]
fn test_enum_required_property() {
    let schema = json!({
        "properties": {
            "mode": {"type": "string", "enum": ["a", "b"]}
        },
        "required": ["mode"]
    });
    let result = schema_to_clap_args(&schema).unwrap();
    // Enum properties obey required just like standard args.
    // Per Python implementation: required is set to false at clap level (STDIN deferral);
    // the [required] annotation is added to help text instead.
    // Rust approach: also set required=false; help text carries "[required]".
    let arg = find_arg(&result.args, "mode").unwrap();
    assert!(!arg.is_required_set(), "required enforced post-parse, not at clap level");
}

#[test]
fn test_enum_stored_in_enum_maps() {
    let schema = json!({
        "properties": {
            "priority": {"type": "integer", "enum": [1, 2, 3]}
        }
    });
    let result = schema_to_clap_args(&schema).unwrap();
    assert!(result.enum_maps.contains_key("priority"));
    let map = &result.enum_maps["priority"];
    assert_eq!(map.len(), 3);
}
```

Run `cargo test test_enum` — all fail.

---

## GREEN — Implement

Remove the `continue` placeholder in `schema_to_clap_args` for the enum branch and replace it. Note: the enum branch comes after the boolean branch (boolean `continue`s first, so booleans never reach this code):

```rust
if let Some(enum_values) = prop_schema.get("enum").and_then(|v| v.as_array()) {
    let flag_long = prop_name.replace('_', "-");
    let is_required = required_list.contains(&prop_name.as_str());
    let help_text = extract_help(prop_schema);
    let default_val = prop_schema.get("default");

    if enum_values.is_empty() {
        warn!("Empty enum for property '{}', no values allowed.", prop_name);
        // Fall through to plain string arg below.
    } else {
        // Convert all values to String for clap.
        let string_values: Vec<String> = enum_values
            .iter()
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect();

        // Store original typed values for reconversion.
        enum_maps.insert(prop_name.clone(), enum_values.to_vec());

        let mut arg = Arg::new(prop_name.clone())
            .long(flag_long)
            .value_parser(clap::builder::PossibleValuesParser::new(&string_values))
            .required(false); // required enforced post-parse for STDIN compatibility

        if let Some(help) = &help_text {
            let help_with_required = if is_required {
                format!("{} [required]", help)
            } else {
                help.clone()
            };
            arg = arg.help(help_with_required);
        } else if is_required {
            arg = arg.help("[required]");
        }

        if let Some(dv) = default_val {
            let dv_str = match dv {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            arg = arg.default_value(dv_str);
        }

        args.push(arg);
        continue;
    }
}
```

After the enum block (when enum is empty or not present), execution falls through to the standard `map_type` Arg builder from the `type-mapping` task.

Note on `required` behaviour: per the Python implementation, required fields are not enforced at the clap level (`required=False` in Python) to allow `--input -` (STDIN) to satisfy them. The `[required]` annotation is added to help text as a hint. The same pattern is followed here: enum args always have `.required(false)`, but `[required]` appears in the help string.

---

## REFACTOR

- The `string_values` slice reference in `PossibleValuesParser::new(&string_values)` requires that `string_values` owns the data. Confirm the borrow checker is satisfied. If lifetime issues arise, convert to `clap::builder::PossibleValuesParser::new(string_values.iter().map(String::as_str).collect::<Vec<_>>())`.
- Run `cargo clippy -- -D warnings`.

---

## Verification

```bash
cargo test test_enum 2>&1
# Expected: test result: ok. 8 passed; 0 failed
```
