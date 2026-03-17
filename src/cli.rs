// apcore-cli — Core CLI dispatcher.
// Protocol spec: FE-01 (LazyModuleGroup equivalent, build_module_command,
//                        collect_input, validate_module_id, set_audit_logger)

use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use thiserror::Error;

use crate::security::AuditLogger;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced by CLI dispatch operations.
#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid module id: {0}")]
    InvalidModuleId(String),

    #[error("stdin read error: {0}")]
    StdinRead(String),

    #[error("json parse error: {0}")]
    JsonParse(String),

    #[error("input too large (limit {limit} bytes, got {actual} bytes)")]
    InputTooLarge { limit: usize, actual: usize },

    #[error("expected JSON object, got a different type")]
    NotAnObject,
}

// ---------------------------------------------------------------------------
// Global audit logger (module-level singleton, set once at startup)
// ---------------------------------------------------------------------------

static AUDIT_LOGGER: Mutex<Option<AuditLogger>> = Mutex::new(None);

/// Set (or clear) the global audit logger used by all module commands.
///
/// Pass `None` to disable auditing. Typically called once during CLI
/// initialisation, before any commands are dispatched.
pub fn set_audit_logger(audit_logger: Option<AuditLogger>) {
    // TODO: store audit_logger in AUDIT_LOGGER mutex.
    let _ = audit_logger;
    todo!("set_audit_logger: store into AUDIT_LOGGER")
}

// ---------------------------------------------------------------------------
// LazyModuleGroup — lazy command builder
// ---------------------------------------------------------------------------

/// Lazy command registry: builds module subcommands on-demand from the
/// apcore Registry, caching them after first construction.
///
/// This is the Rust equivalent of the Python `LazyModuleGroup` (Click group
/// subclass with lazy `get_command` / `list_commands`).
pub struct LazyModuleGroup {
    // TODO: hold Arc<dyn Registry>, Arc<dyn Executor>, command cache HashMap
}

impl LazyModuleGroup {
    /// Create a new lazy module group.
    ///
    /// # Arguments
    /// * `registry` — apcore module registry
    /// * `executor` — apcore executor for running modules
    pub fn new(/* registry: Arc<dyn Registry>, executor: Arc<dyn Executor> */) -> Self {
        // TODO: initialise fields
        todo!("LazyModuleGroup::new")
    }

    /// Return the list of available command names (builtins + module ids).
    pub fn list_commands(&self) -> Vec<String> {
        // TODO: query registry.list() and prepend builtins
        //       ["exec", "list", "describe", "completion", "man"]
        todo!("LazyModuleGroup::list_commands")
    }

    /// Look up a command by name, building and caching it if it is a module.
    pub fn get_command(&mut self, name: &str) -> Option<clap::Command> {
        // TODO: check built-in map first, then registry
        todo!("LazyModuleGroup::get_command: name={name}")
    }
}

// ---------------------------------------------------------------------------
// build_module_command
// ---------------------------------------------------------------------------

/// Build a clap `Command` for a single module definition.
///
/// The resulting subcommand has:
/// * its `name` set to `module_def.module_id`
/// * its `about` set to `module_def.description`
/// * one `--input` flag for piped JSON
/// * schema-derived flags from `schema_to_clap_args`
/// * an execution callback that calls `executor.execute`
pub fn build_module_command(
    // module_def: &apcore::ModuleDescriptor,
    // executor: Arc<dyn apcore::Executor>,
) -> clap::Command {
    // TODO: call schema_to_clap_args on module_def.input_schema,
    //       attach --input/--auto-approve flags, wire execution callback.
    todo!("build_module_command")
}

// ---------------------------------------------------------------------------
// collect_input
// ---------------------------------------------------------------------------

const STDIN_SIZE_LIMIT_BYTES: usize = 10 * 1024 * 1024; // 10 MiB

/// Inner implementation: accepts any `Read` source for testability.
///
/// # Arguments
/// * `stdin_flag`  — `Some("-")` to read from `reader`, anything else skips STDIN
/// * `cli_kwargs`  — map of flag name → value (`Null` values are dropped)
/// * `large_input` — if `false`, reject payloads exceeding `STDIN_SIZE_LIMIT_BYTES`
/// * `reader`      — byte source to read from when `stdin_flag == Some("-")`
///
/// # Errors
/// Returns `CliError` on oversized input, invalid JSON, or non-object JSON.
pub fn collect_input_from_reader<R: Read>(
    stdin_flag: Option<&str>,
    cli_kwargs: HashMap<String, Value>,
    large_input: bool,
    mut reader: R,
) -> Result<HashMap<String, Value>, CliError> {
    // Drop Null values from CLI kwargs.
    let cli_non_null: HashMap<String, Value> = cli_kwargs
        .into_iter()
        .filter(|(_, v)| !v.is_null())
        .collect();

    if stdin_flag != Some("-") {
        return Ok(cli_non_null);
    }

    let mut buf = Vec::new();
    reader
        .read_to_end(&mut buf)
        .map_err(|e| CliError::StdinRead(e.to_string()))?;

    if !large_input && buf.len() > STDIN_SIZE_LIMIT_BYTES {
        return Err(CliError::InputTooLarge {
            limit: STDIN_SIZE_LIMIT_BYTES,
            actual: buf.len(),
        });
    }

    if buf.is_empty() {
        return Ok(cli_non_null);
    }

    let stdin_value: Value =
        serde_json::from_slice(&buf).map_err(|e| CliError::JsonParse(e.to_string()))?;

    let stdin_map = match stdin_value {
        Value::Object(m) => m,
        _ => return Err(CliError::NotAnObject),
    };

    // Merge: STDIN base, CLI kwargs override on collision.
    let mut merged: HashMap<String, Value> = stdin_map.into_iter().collect();
    merged.extend(cli_non_null);
    Ok(merged)
}

/// Merge CLI keyword arguments with optional STDIN JSON.
///
/// Resolution order (highest priority first):
/// 1. CLI flags (non-`Null` values in `cli_kwargs`)
/// 2. STDIN JSON (when `stdin_flag` is `Some("-")`)
///
/// # Arguments
/// * `stdin_flag`  — `Some("-")` to read from STDIN, `None` to skip
/// * `cli_kwargs`  — map of flag name → value (`Null` values are ignored)
/// * `large_input` — if `false`, reject STDIN payloads exceeding 10 MiB
///
/// # Errors
/// Returns `CliError` (exit code 2) on oversized input, invalid JSON, or
/// non-object JSON.
pub fn collect_input(
    stdin_flag: Option<&str>,
    cli_kwargs: HashMap<String, Value>,
    large_input: bool,
) -> Result<HashMap<String, Value>, CliError> {
    collect_input_from_reader(stdin_flag, cli_kwargs, large_input, std::io::stdin())
}

// ---------------------------------------------------------------------------
// validate_module_id
// ---------------------------------------------------------------------------

const MODULE_ID_MAX_LEN: usize = 128;

/// Validate a module identifier.
///
/// # Rules
/// * Maximum 128 characters
/// * Matches `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$`
/// * No leading/trailing dots, no consecutive dots
/// * Must not start with a digit or uppercase letter
///
/// # Errors
/// Returns `CliError::InvalidModuleId` (exit code 2) on any violation.
pub fn validate_module_id(module_id: &str) -> Result<(), CliError> {
    if module_id.len() > MODULE_ID_MAX_LEN {
        return Err(CliError::InvalidModuleId(format!(
            "Invalid module ID format: '{module_id}'. Maximum length is {MODULE_ID_MAX_LEN} characters."
        )));
    }
    if !is_valid_module_id(module_id) {
        return Err(CliError::InvalidModuleId(format!(
            "Invalid module ID format: '{module_id}'."
        )));
    }
    Ok(())
}

/// Hand-written validator matching `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$`.
///
/// Does not require the `regex` crate.
#[inline]
fn is_valid_module_id(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // Split on '.' and validate each segment individually.
    for segment in s.split('.') {
        if segment.is_empty() {
            // Catches leading dot, trailing dot, and consecutive dots.
            return false;
        }
        let mut chars = segment.chars();
        // First character must be a lowercase ASCII letter.
        match chars.next() {
            Some(c) if c.is_ascii_lowercase() => {}
            _ => return false,
        }
        // Remaining characters: lowercase letter, ASCII digit, or underscore.
        for c in chars {
            if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '_' {
                return false;
            }
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_module_id_valid() {
        // Valid IDs must not return an error.
        for id in ["math.add", "text.summarize", "a", "a.b.c"] {
            let result = validate_module_id(id);
            assert!(result.is_ok(), "expected ok for '{id}': {result:?}");
        }
    }

    #[test]
    fn test_validate_module_id_too_long() {
        let long_id = "a".repeat(129);
        assert!(validate_module_id(&long_id).is_err());
    }

    #[test]
    fn test_validate_module_id_invalid_format() {
        for id in ["INVALID!ID", "123abc", ".leading.dot", "a..b", "a."] {
            assert!(
                validate_module_id(id).is_err(),
                "expected error for '{id}'"
            );
        }
    }

    #[test]
    fn test_validate_module_id_max_length() {
        let max_id = "a".repeat(128);
        assert!(validate_module_id(&max_id).is_ok());
    }

    #[test]
    fn test_set_audit_logger_none() {
        // Setting None should not panic.
        // assert!(false, "not implemented");
        // TODO: uncomment and implement
    }

    // collect_input tests (TDD red → green)

    #[test]
    fn test_collect_input_no_stdin_drops_null_values() {
        use serde_json::json;
        let mut kwargs = HashMap::new();
        kwargs.insert("a".to_string(), json!(5));
        kwargs.insert("b".to_string(), Value::Null);

        let result = collect_input(None, kwargs, false).unwrap();
        assert_eq!(result.get("a"), Some(&json!(5)));
        assert!(!result.contains_key("b"), "Null values must be dropped");
    }

    #[test]
    fn test_collect_input_stdin_valid_json() {
        use serde_json::json;
        use std::io::Cursor;
        let stdin_bytes = b"{\"x\": 42}";
        let reader = Cursor::new(stdin_bytes.to_vec());
        let result = collect_input_from_reader(Some("-"), HashMap::new(), false, reader).unwrap();
        assert_eq!(result.get("x"), Some(&json!(42)));
    }

    #[test]
    fn test_collect_input_cli_overrides_stdin() {
        use serde_json::json;
        use std::io::Cursor;
        let stdin_bytes = b"{\"a\": 5}";
        let reader = Cursor::new(stdin_bytes.to_vec());
        let mut kwargs = HashMap::new();
        kwargs.insert("a".to_string(), json!(99));
        let result = collect_input_from_reader(Some("-"), kwargs, false, reader).unwrap();
        assert_eq!(result.get("a"), Some(&json!(99)), "CLI must override STDIN");
    }

    #[test]
    fn test_collect_input_oversized_stdin_rejected() {
        use std::io::Cursor;
        let big = vec![b' '; 10 * 1024 * 1024 + 1];
        let reader = Cursor::new(big);
        let err =
            collect_input_from_reader(Some("-"), HashMap::new(), false, reader).unwrap_err();
        assert!(matches!(err, CliError::InputTooLarge { .. }));
    }

    #[test]
    fn test_collect_input_large_input_allowed() {
        use std::io::Cursor;
        let mut payload = b"{\"k\": \"".to_vec();
        payload.extend(vec![b'x'; 11 * 1024 * 1024]);
        payload.extend(b"\"}");
        let reader = Cursor::new(payload);
        let result = collect_input_from_reader(Some("-"), HashMap::new(), true, reader);
        assert!(result.is_ok(), "large_input=true must accept oversized payload");
    }

    #[test]
    fn test_collect_input_invalid_json_returns_error() {
        use std::io::Cursor;
        let reader = Cursor::new(b"not json at all".to_vec());
        let err =
            collect_input_from_reader(Some("-"), HashMap::new(), false, reader).unwrap_err();
        assert!(matches!(err, CliError::JsonParse(_)));
    }

    #[test]
    fn test_collect_input_non_object_json_returns_error() {
        use std::io::Cursor;
        let reader = Cursor::new(b"[1, 2, 3]".to_vec());
        let err =
            collect_input_from_reader(Some("-"), HashMap::new(), false, reader).unwrap_err();
        assert!(matches!(err, CliError::NotAnObject));
    }

    #[test]
    fn test_collect_input_empty_stdin_returns_empty_map() {
        use std::io::Cursor;
        let reader = Cursor::new(b"".to_vec());
        let result =
            collect_input_from_reader(Some("-"), HashMap::new(), false, reader).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_collect_input_no_stdin_flag_returns_cli_kwargs() {
        use serde_json::json;
        let mut kwargs = HashMap::new();
        kwargs.insert("foo".to_string(), json!("bar"));
        let result = collect_input(None, kwargs.clone(), false).unwrap();
        assert_eq!(result.get("foo"), Some(&json!("bar")));
    }
}
