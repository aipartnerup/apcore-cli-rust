# Task: boolean-flag-pairs

**Feature**: FE-02 Schema Parser
**File**: `src/schema_parser.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~2h
**Depends on**: `type-mapping`
**Required by**: `help-text-and-collision`

---

## Context

Properties with `"type": "boolean"` must produce a `--flag` / `--no-flag` pair rather than a single `--flag` with a string value. The Python implementation uses `click.Option(["--verbose/--no-verbose"])`. Clap v4 has no built-in equivalent; the approach is:

1. Create two separate `clap::Arg`s:
   - `--<flag>` with `ArgAction::SetTrue`
   - `--no-<flag>` with `ArgAction::SetFalse`
2. Record a `BoolFlagPair` in `SchemaArgs.bool_pairs` so `collect_input` (in `cli.rs`) can reconcile the pair into a single `bool` value.

The `id` of both Args must be distinct (`"<prop_name>"` and `"no-<prop_name>"`) so clap does not raise a duplicate-id error. The reconciliation in `collect_input` is: if `--flag` is present → `true`; if `--no-flag` is present → `false`; if neither → `BoolFlagPair.default_val`.

Edge case from spec: if `"enum": [true]` appears alongside `"type": "boolean"`, treat as a standard boolean flag pair (ignore the enum constraint).

---

## RED — Write Failing Tests First

Add to `tests/test_schema_parser.rs`:

```rust
#[test]
fn test_boolean_flag_pair_produced() {
    let schema = json!({
        "properties": {"verbose": {"type": "boolean"}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    // Both --verbose and --no-verbose must be in args.
    assert!(
        find_arg(&result.args, "verbose").is_some(),
        "--verbose must be present"
    );
    assert!(
        find_arg(&result.args, "no-verbose").is_some(),
        "--no-verbose must be present"
    );
}

#[test]
fn test_boolean_pair_actions() {
    let schema = json!({
        "properties": {"verbose": {"type": "boolean"}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let pos_arg = find_arg(&result.args, "verbose").unwrap();
    let neg_arg = find_arg(&result.args, "no-verbose").unwrap();
    assert_eq!(pos_arg.get_action(), &clap::ArgAction::SetTrue);
    assert_eq!(neg_arg.get_action(), &clap::ArgAction::SetFalse);
}

#[test]
fn test_boolean_default_false() {
    let schema = json!({
        "properties": {"debug": {"type": "boolean"}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let pair = result.bool_pairs.iter().find(|p| p.prop_name == "debug");
    assert!(pair.is_some());
    assert!(!pair.unwrap().default_val, "default must be false when not specified");
}

#[test]
fn test_boolean_default_true() {
    let schema = json!({
        "properties": {"enabled": {"type": "boolean", "default": true}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let pair = result
        .bool_pairs
        .iter()
        .find(|p| p.prop_name == "enabled")
        .expect("BoolFlagPair must be recorded");
    assert!(pair.default_val, "default must be true when schema says true");
}

#[test]
fn test_boolean_pair_recorded_in_bool_pairs() {
    let schema = json!({
        "properties": {"dry_run": {"type": "boolean"}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let pair = result.bool_pairs.iter().find(|p| p.prop_name == "dry_run");
    assert!(pair.is_some(), "BoolFlagPair must be recorded for dry_run");
    assert_eq!(
        pair.unwrap().flag_long,
        "dry-run",
        "flag_long must use hyphen form"
    );
}

#[test]
fn test_boolean_underscore_to_hyphen() {
    let schema = json!({
        "properties": {"dry_run": {"type": "boolean"}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    assert!(find_arg(&result.args, "dry-run").is_some(), "--dry-run");
    assert!(find_arg(&result.args, "no-dry-run").is_some(), "--no-dry-run");
}

#[test]
fn test_boolean_with_enum_true_treated_as_flag() {
    // Boolean with enum: [true] must still produce a flag pair, not an enum choice.
    let schema = json!({
        "properties": {"strict": {"type": "boolean", "enum": [true]}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    assert!(find_arg(&result.args, "strict").is_some());
    assert!(find_arg(&result.args, "no-strict").is_some());
    // Must NOT be in enum_maps.
    assert!(!result.enum_maps.contains_key("strict"));
}

#[test]
fn test_boolean_not_counted_as_required_arg() {
    // Booleans become two args but neither must be `required`.
    let schema = json!({
        "properties": {"active": {"type": "boolean"}},
        "required": ["active"]
    });
    let result = schema_to_clap_args(&schema).unwrap();
    // Booleans are never required at the clap level (default covers absent case).
    let pos = find_arg(&result.args, "active").unwrap();
    let neg = find_arg(&result.args, "no-active").unwrap();
    assert!(!pos.is_required_set());
    assert!(!neg.is_required_set());
}
```

Run `cargo test test_boolean` — all fail.

---

## GREEN — Implement

Remove the `continue` placeholder in `schema_to_clap_args` for the boolean branch and replace it:

```rust
if schema_type == Some("boolean") {
    let flag_long = prop_name.replace('_', "-");
    let default_val = prop_schema
        .get("default")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let help_text = extract_help(prop_schema);

    let mut pos_arg = Arg::new(prop_name.clone())
        .long(flag_long.clone())
        .action(ArgAction::SetTrue);

    let mut neg_arg = Arg::new(format!("no-{}", prop_name))
        .long(format!("no-{}", flag_long))
        .action(ArgAction::SetFalse);

    if let Some(ref help) = help_text {
        pos_arg = pos_arg.help(help.clone());
        neg_arg = neg_arg.help(format!("Disable --{flag_long}"));
    }

    args.push(pos_arg);
    args.push(neg_arg);

    bool_pairs.push(BoolFlagPair {
        prop_name: prop_name.clone(),
        flag_long,
        default_val,
    });

    continue;
}
```

The `enum: [true]` edge case is handled by placing the boolean `type` check before the general `enum` check: since `schema_type == Some("boolean")` fires first and `continue`s, the enum branch is never reached for boolean properties.

---

## REFACTOR

- Ensure `--no-<flag>` long name for properties with underscores is `--no-<hyphen-form>` (e.g., `dry_run` → `--no-dry-run`), confirmed by the `test_boolean_underscore_to_hyphen` test.
- Run `cargo clippy -- -D warnings`.

---

## Verification

```bash
cargo test test_boolean 2>&1
# Expected: test result: ok. 8 passed; 0 failed
```
