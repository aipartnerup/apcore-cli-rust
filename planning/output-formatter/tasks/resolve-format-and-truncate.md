# Task: resolve-format-and-truncate

**Feature**: FE-08 Output Formatter
**File**: `src/output.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~1h
**Depends on**: (none)
**Required by**: `format-module-list`, `format-module-detail`, `format-exec-result`

---

## Context

`resolve_format` and `truncate` are the foundational helpers for the entire output module. Python's `resolve_format` calls `sys.stdout.isatty()` directly; the Rust version wraps the TTY check in a private `resolve_format_inner` that accepts a `bool` so tests can inject both states without spawning a subprocess.

`truncate` is a pure string helper: if the input is longer than `max_length`, return the first `max_length - 3` characters followed by `"..."`.

The existing `output.rs` stub has `todo!("resolve_format")`. This task replaces it with a working implementation and adds the `truncate` helper.

---

## RED — Write Failing Tests First

Add the following tests to the `#[cfg(test)]` block at the bottom of `src/output.rs`. All must compile but fail before GREEN.

```rust
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
}
```

Run `cargo test test_resolve_format test_truncate -- --test-output immediate` to confirm all fail with `todo!` panics.

---

## GREEN — Implement

Replace the `resolve_format` stub and add `resolve_format_inner` and `truncate` in `src/output.rs`:

```rust
use std::io::IsTerminal;

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
/// If truncation occurs, the last 3 characters are replaced with `"..."`.
pub(crate) fn truncate(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        return text.to_string();
    }
    let cutoff = max_length.saturating_sub(3);
    format!("{}...", &text[..cutoff])
}
```

**Notes:**
- `pub(crate)` on `truncate` keeps it internal; `resolve_format_inner` is `pub(crate)` so integration tests in `tests/test_output.rs` can import it with `use apcore_cli::output::resolve_format_inner` after adding a `pub(crate)` re-export or by testing via the inline module.
- If `tests/test_output.rs` needs to call `resolve_format_inner`, change its visibility to `pub` and add it to the `lib.rs` re-export list. The inline unit tests in `src/output.rs` already have access via `super::`.
- `std::io::IsTerminal` is stable since Rust 1.70; no feature flag needed.

---

## REFACTOR

- Confirm `truncate` is Unicode-safe for the project's use case. The Python `len(text)` counts characters (Unicode code points), but the current Rust implementation uses byte offsets (`&text[..cutoff]`). If `description` fields are guaranteed to be ASCII (module IDs and short descriptions), byte slicing is correct. If Unicode is required, replace `text.len()` with `text.chars().count()` and rebuild the truncated string using `text.chars().take(cutoff).collect::<String>()`. Document the choice.
- Run `cargo clippy -- -D warnings` on `src/output.rs`.
- Remove the four `assert!(false, "not implemented")` stubs from the inline `#[cfg(test)]` block in `src/output.rs` for `resolve_format_explicit_json` and `resolve_format_explicit_table` (they are replaced by the new tests above).

---

## Verification

```bash
cargo test test_resolve_format test_truncate 2>&1
# Expected: 8 tests pass, 0 fail.
```
