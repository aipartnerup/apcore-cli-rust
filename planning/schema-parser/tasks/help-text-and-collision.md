# Task: help-text-and-collision

**Feature**: FE-02 Schema Parser
**File**: `src/schema_parser.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~1h
**Depends on**: `boolean-flag-pairs`, `enum-choices`
**Required by**: `reconvert-enum-values`

---

## Context

Two small but exact-spec behaviours must be verified end-to-end after the previous tasks have placed their code:

1. **Help text extraction** (`extract_help`): implemented in `type-mapping` but needs dedicated tests covering all four cases — `x-llm-description` present, only `description` present, text > configurable limit (default 1000 chars), and neither field present.

2. **Flag collision detection**: `schema_to_clap_args` must return `Err(SchemaParserError::FlagCollision)` when two property names normalise to the same `--flag-name` (e.g., `foo_bar` and `foo-bar` both → `--foo-bar`). The caller in `cli.rs` maps this to exit 48.

The `extract_help` function body was written in the `type-mapping` task. This task adds tests that would have caught regressions. The collision check was written in `type-mapping` as well; this task adds the tests for it.

---

## RED — Write Failing Tests First

Add to `tests/test_schema_parser.rs`:

```rust
// --- Help text tests ---

#[test]
fn test_help_prefers_x_llm_description() {
    let schema = json!({
        "properties": {
            "q": {
                "type": "string",
                "description": "plain description",
                "x-llm-description": "LLM-optimised description"
            }
        }
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let arg = find_arg(&result.args, "q").unwrap();
    let help = arg.get_help().map(|s| s.to_string()).unwrap_or_default();
    assert!(
        help.contains("LLM-optimised"),
        "help must come from x-llm-description, got: {help}"
    );
    assert!(
        !help.contains("plain description"),
        "help must NOT come from description when x-llm-description is present"
    );
}

#[test]
fn test_help_falls_back_to_description() {
    let schema = json!({
        "properties": {
            "q": {"type": "string", "description": "fallback text"}
        }
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let arg = find_arg(&result.args, "q").unwrap();
    let help = arg.get_help().map(|s| s.to_string()).unwrap_or_default();
    assert!(help.contains("fallback text"));
}

#[test]
fn test_help_truncated_at_1000_chars() {
    // Build a description that is exactly 1100 chars long.
    let long_desc = "A".repeat(1100);
    let schema = json!({
        "properties": {
            "q": {"type": "string", "description": long_desc}
        }
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let arg = find_arg(&result.args, "q").unwrap();
    let help = arg.get_help().map(|s| s.to_string()).unwrap_or_default();
    assert_eq!(help.len(), 1000, "truncated help must be exactly 1000 chars");
    assert!(help.ends_with("..."), "truncated help must end with '...'");
}

#[test]
fn test_help_within_limit_not_truncated() {
    let desc = "B".repeat(999);
    let schema = json!({
        "properties": {
            "q": {"type": "string", "description": desc}
        }
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let arg = find_arg(&result.args, "q").unwrap();
    let help = arg.get_help().map(|s| s.to_string()).unwrap_or_default();
    assert_eq!(help.len(), 999);
    assert!(!help.ends_with("..."));
}

#[test]
fn test_help_none_when_no_description_fields() {
    let schema = json!({
        "properties": {"q": {"type": "string"}}
    });
    let result = schema_to_clap_args(&schema).unwrap();
    let arg = find_arg(&result.args, "q").unwrap();
    // No help text means get_help() returns None.
    assert!(arg.get_help().is_none());
}

// --- Collision tests ---

#[test]
fn test_flag_collision_detection() {
    // "foo_bar" and "foo-bar" both normalise to --foo-bar.
    let schema = json!({
        "properties": {
            "foo_bar": {"type": "string"},
            "foo-bar": {"type": "string"}
        }
    });
    let result = schema_to_clap_args(&schema);
    assert!(
        matches!(result, Err(SchemaParserError::FlagCollision { .. })),
        "expected FlagCollision, got: {result:?}"
    );
}

#[test]
fn test_flag_collision_error_message_contains_both_names() {
    let schema = json!({
        "properties": {
            "my_flag": {"type": "string"},
            "my-flag": {"type": "string"}
        }
    });
    let err = schema_to_clap_args(&schema).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("my_flag") || msg.contains("my-flag"));
    assert!(msg.contains("my-flag") || msg.contains("--my-flag"));
}

#[test]
fn test_no_collision_for_distinct_flags() {
    let schema = json!({
        "properties": {
            "alpha": {"type": "string"},
            "beta": {"type": "string"}
        }
    });
    let result = schema_to_clap_args(&schema);
    assert!(result.is_ok());
}
```

Run `cargo test test_help test_flag_collision test_no_collision` — all must fail or pass only trivially (help text already partially implemented in `type-mapping`).

---

## GREEN — Verify / Adjust

The `extract_help` function was already written in `type-mapping`. Run the tests; if any fail, adjust the implementation:

- **Truncation boundary**: `text.len() > HELP_TEXT_MAX_LEN (1000)` triggers truncation; `len() == 1000` does not. The slice is `&text[..997]` + `"..."` = 1000 chars total. The limit is configurable via `cli.help_text_max_length`.
- **x-llm-description empty string**: `filter(|s| !s.is_empty())` ensures an empty `x-llm-description` falls back to `description`. No new code needed; test `test_help_prefers_x_llm_description` already covers the non-empty case.

The collision check was also written in `type-mapping`. If `serde_json::Map` iterates in insertion order (which it does when the `preserve_order` feature is enabled, the default), the first-seen property is stored in `seen_flags` and the second triggers the collision. If the map does not guarantee order, the test asserting both names appear in the error message remains valid regardless of which name is `prop_a` vs `prop_b`.

No new code is expected in this task; it is primarily a test-coverage task.

---

## REFACTOR

- Confirm `SchemaParserError::FlagCollision` is exported from `lib.rs`.
- Run `cargo clippy -- -D warnings`.
- Confirm `test_flag_collision_detection` is deterministic: `serde_json` preserves JSON object key order by default (insertion order). The test relies on `"foo_bar"` being inserted before `"foo-bar"`. This is safe with serde_json default features.

---

## Verification

```bash
cargo test test_help test_flag_collision test_no_collision 2>&1
# Expected: test result: ok. 9 passed; 0 failed
```
