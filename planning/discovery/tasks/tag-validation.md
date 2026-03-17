# Task: tag-validation

**Feature**: FE-04 Discovery
**File**: `src/discovery.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~1h
**Depends on**: (none — foundational task)
**Required by**: `list-command`, `describe-command`

---

## Context

Tag validation is a prerequisite for both `list_command` and `describe_command`. The `list` command validates each value passed to `--tag` against the pattern `^[a-z][a-z0-9_-]*$`. An invalid tag format exits 2; a valid but non-existent tag produces an empty filtered result with exit 0.

This task also defines the `RegistryProvider` trait and a `MockRegistry` for use in unit tests across all discovery tasks. Without a mock registry the command handlers cannot be tested without a live apcore registry.

This task also defines the `DiscoveryError` enum used by `list_command` and `describe_command` to return structured errors to the caller instead of calling `std::process::exit` directly.

---

## RED — Write Failing Tests First

Add to `src/discovery.rs` inline `#[cfg(test)]` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // --- validate_tag ---

    #[test]
    fn test_validate_tag_valid_simple() {
        assert!(validate_tag("math"), "single lowercase word must be valid");
    }

    #[test]
    fn test_validate_tag_valid_with_digits_and_dash() {
        assert!(validate_tag("ml-v2"), "digits and dash must be valid");
    }

    #[test]
    fn test_validate_tag_valid_with_underscore() {
        assert!(validate_tag("core_util"), "underscore must be valid");
    }

    #[test]
    fn test_validate_tag_invalid_uppercase() {
        assert!(!validate_tag("Math"), "uppercase start must be invalid");
    }

    #[test]
    fn test_validate_tag_invalid_starts_with_digit() {
        assert!(!validate_tag("1tag"), "digit start must be invalid");
    }

    #[test]
    fn test_validate_tag_invalid_special_chars() {
        assert!(!validate_tag("invalid!"), "special chars must be invalid");
    }

    #[test]
    fn test_validate_tag_invalid_empty() {
        assert!(!validate_tag(""), "empty string must be invalid");
    }

    #[test]
    fn test_validate_tag_invalid_space() {
        assert!(!validate_tag("has space"), "space must be invalid");
    }

    // --- RegistryProvider / MockRegistry ---

    #[test]
    fn test_mock_registry_list_returns_ids() {
        let registry = MockRegistry::new(vec![
            mock_module("math.add", "Add numbers", &["math", "core"]),
            mock_module("text.upper", "Uppercase text", &["text"]),
        ]);
        let ids = registry.list();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"math.add".to_string()));
    }

    #[test]
    fn test_mock_registry_get_definition_found() {
        let registry = MockRegistry::new(vec![
            mock_module("math.add", "Add numbers", &["math"]),
        ]);
        let def = registry.get_definition("math.add");
        assert!(def.is_some());
        assert_eq!(def.unwrap()["module_id"], "math.add");
    }

    #[test]
    fn test_mock_registry_get_definition_not_found() {
        let registry = MockRegistry::new(vec![]);
        assert!(registry.get_definition("non.existent").is_none());
    }
}
```

Run `cargo test test_validate_tag` and `cargo test test_mock_registry` — all fail (symbols not yet defined).

---

## GREEN — Implement

Replace the contents of `src/discovery.rs` with:

```rust
// apcore-cli — Discovery subcommands (list + describe).
// Protocol spec: FE-04

use std::sync::Arc;

use clap::{Arg, ArgAction, Command};
use serde_json::Value;
use thiserror::Error;

// ---------------------------------------------------------------------------
// DiscoveryError
// ---------------------------------------------------------------------------

/// Errors produced by discovery command handlers.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("module '{0}' not found")]
    ModuleNotFound(String),

    #[error("invalid module id: {0}")]
    InvalidModuleId(String),

    #[error("invalid tag format: '{0}'. Tags must match [a-z][a-z0-9_-]*.")]
    InvalidTag(String),
}

// ---------------------------------------------------------------------------
// RegistryProvider trait
// ---------------------------------------------------------------------------

/// Minimal registry interface used by discovery commands.
///
/// The real `apcore::Registry` implements this trait via a thin adaptor
/// (added in the `core-dispatcher` feature). Tests use `MockRegistry`.
pub trait RegistryProvider: Send + Sync {
    /// Return all module IDs in the registry.
    fn list(&self) -> Vec<String>;

    /// Return the JSON descriptor for a single module, or `None` if not found.
    fn get_definition(&self, id: &str) -> Option<Value>;
}

// ---------------------------------------------------------------------------
// validate_tag
// ---------------------------------------------------------------------------

/// Validate a tag string against the pattern `^[a-z][a-z0-9_-]*$`.
///
/// Returns `true` if valid, `false` otherwise. Does not exit the process.
pub fn validate_tag(tag: &str) -> bool {
    let mut chars = tag.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

// ---------------------------------------------------------------------------
// register_discovery_commands
// ---------------------------------------------------------------------------

/// Attach `list` and `describe` subcommands to the given root command.
///
/// Returns the root command with the subcommands added. Follows the clap v4
/// builder idiom (commands are consumed and returned, not mutated in-place).
pub fn register_discovery_commands(
    cli: Command,
    registry: Arc<dyn RegistryProvider>,
) -> Command {
    cli.subcommand(list_command(Arc::clone(&registry)))
       .subcommand(describe_command(registry))
}

// ---------------------------------------------------------------------------
// list_command / cmd_list
// ---------------------------------------------------------------------------

fn list_command(registry: Arc<dyn RegistryProvider>) -> Command {
    Command::new("list")
        .about("List available modules in the registry")
        .arg(
            Arg::new("tag")
                .long("tag")
                .action(ArgAction::Append)
                .value_name("TAG")
                .help("Filter modules by tag (AND logic). Repeatable."),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .value_parser(clap::builder::PossibleValuesParser::new(["table", "json"]))
                .value_name("FORMAT")
                .help("Output format. Default: table (TTY) or json (non-TTY)."),
        )
}

/// Execute the `list` subcommand logic.
///
/// Returns `Ok(String)` with the formatted output on success.
/// Returns `Err(DiscoveryError)` on invalid tag format.
///
/// The caller is responsible for printing the output and mapping errors to
/// exit codes:  `DiscoveryError::InvalidTag` → exit 2.
pub fn cmd_list(
    registry: &dyn RegistryProvider,
    tags: &[&str],
    explicit_format: Option<&str>,
) -> Result<String, DiscoveryError> {
    // Validate all tag formats before filtering.
    for tag in tags {
        if !validate_tag(tag) {
            return Err(DiscoveryError::InvalidTag(tag.to_string()));
        }
    }

    // Collect all module definitions.
    let mut modules: Vec<Value> = registry
        .list()
        .into_iter()
        .filter_map(|id| registry.get_definition(&id))
        .collect();

    // Apply AND tag filter if any tags were specified.
    if !tags.is_empty() {
        modules.retain(|m| {
            let mod_tags: Vec<&str> = m
                .get("tags")
                .and_then(|t| t.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            tags.iter().all(|required| mod_tags.contains(required))
        });
    }

    let fmt = crate::output::resolve_format(explicit_format);
    Ok(crate::output::format_module_list(&modules, fmt, tags))
}

// ---------------------------------------------------------------------------
// describe_command / cmd_describe
// ---------------------------------------------------------------------------

fn describe_command(registry: Arc<dyn RegistryProvider>) -> Command {
    Command::new("describe")
        .about("Show metadata, schema, and annotations for a module")
        .arg(
            Arg::new("module_id")
                .required(true)
                .value_name("MODULE_ID")
                .help("Canonical module identifier (e.g. math.add)"),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .value_parser(clap::builder::PossibleValuesParser::new(["table", "json"]))
                .value_name("FORMAT")
                .help("Output format. Default: table (TTY) or json (non-TTY)."),
        )
}

/// Execute the `describe` subcommand logic.
///
/// Returns `Ok(String)` with the formatted output on success.
/// Returns `Err(DiscoveryError)` on invalid module ID or module not found.
///
/// Exit code mapping for the caller:
/// - `DiscoveryError::InvalidModuleId` → exit 2
/// - `DiscoveryError::ModuleNotFound`  → exit 44
pub fn cmd_describe(
    registry: &dyn RegistryProvider,
    module_id: &str,
    explicit_format: Option<&str>,
) -> Result<String, DiscoveryError> {
    // Validate module ID format (reuses cli::validate_module_id logic).
    crate::cli::validate_module_id(module_id)
        .map_err(|_| DiscoveryError::InvalidModuleId(module_id.to_string()))?;

    let module = registry
        .get_definition(module_id)
        .ok_or_else(|| DiscoveryError::ModuleNotFound(module_id.to_string()))?;

    let fmt = crate::output::resolve_format(explicit_format);
    Ok(crate::output::format_module_detail(&module, fmt))
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) fn mock_module(id: &str, description: &str, tags: &[&str]) -> Value {
    serde_json::json!({
        "module_id": id,
        "description": description,
        "tags": tags,
    })
}

#[cfg(test)]
pub(crate) struct MockRegistry {
    modules: Vec<Value>,
}

#[cfg(test)]
impl MockRegistry {
    pub fn new(modules: Vec<Value>) -> Self {
        Self { modules }
    }
}

#[cfg(test)]
impl RegistryProvider for MockRegistry {
    fn list(&self) -> Vec<String> {
        self.modules
            .iter()
            .filter_map(|m| m.get("module_id").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect()
    }

    fn get_definition(&self, id: &str) -> Option<Value> {
        self.modules
            .iter()
            .find(|m| m.get("module_id").and_then(|v| v.as_str()) == Some(id))
            .cloned()
    }
}
```

---

## REFACTOR

- Confirm `validate_tag` handles the full tag character set: `[a-z0-9_-]` after the first character.
- Run `cargo clippy -- -D warnings` on `src/discovery.rs`.
- Ensure `mock_module`, `MockRegistry`, and `MockRegistry::new` are `#[cfg(test)]` gated so they do not appear in the release binary.

---

## Verification

```bash
cargo test test_validate_tag 2>&1
# Expected: 8 tests pass, 0 fail.

cargo test test_mock_registry 2>&1
# Expected: 3 tests pass, 0 fail.
```
