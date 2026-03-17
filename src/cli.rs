// apcore-cli — Core CLI dispatcher.
// Protocol spec: FE-01 (LazyModuleGroup equivalent, build_module_command,
//                        collect_input, validate_module_id, set_audit_logger)

use std::collections::HashMap;
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

const DEFAULT_STDIN_LIMIT: usize = 10 * 1024 * 1024; // 10 MiB

/// Merge CLI keyword arguments with optional STDIN JSON.
///
/// Resolution order (highest priority first):
/// 1. CLI flags (non-None values in `cli_kwargs`)
/// 2. STDIN JSON (when `stdin_flag` is `Some("-")`)
///
/// # Arguments
/// * `stdin_flag`  — `Some("-")` to read from STDIN, `None` to skip
/// * `cli_kwargs`  — map of flag name → value (None values are ignored)
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
    // TODO: implement STDIN reading, size check, JSON parse, merge with cli_kwargs.
    let _ = (stdin_flag, cli_kwargs, large_input);
    todo!("collect_input")
}

// ---------------------------------------------------------------------------
// validate_module_id
// ---------------------------------------------------------------------------

const MODULE_ID_MAX_LEN: usize = 128;
// Pattern: lowercase letters, digits, underscores, dots — no leading/trailing
// dot, no consecutive dots, must not start with a digit.
const MODULE_ID_PATTERN: &str = r"^[a-z_][a-z0-9_.]*$";

/// Validate a module identifier.
///
/// # Rules
/// * Maximum 128 characters
/// * Matches `^[a-z_][a-z0-9_.]*$`
/// * No leading/trailing dots
/// * No consecutive dots
///
/// # Errors
/// Returns `CliError::InvalidModuleId` (exit code 2) on any violation.
pub fn validate_module_id(module_id: &str) -> Result<(), CliError> {
    // TODO: enforce length, regex, dot rules.
    let _ = module_id;
    todo!("validate_module_id")
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
    fn test_collect_input_no_stdin() {
        let mut kwargs = HashMap::new();
        kwargs.insert("a".to_string(), Value::Number(5.into()));
        kwargs.insert("b".to_string(), Value::Null);
        let result = collect_input(None, kwargs, false);
        // TODO: remove assert!(false) once implemented
        assert!(false, "not implemented");
        let _ = result;
    }

    #[test]
    fn test_set_audit_logger_none() {
        // Setting None should not panic.
        // assert!(false, "not implemented");
        // TODO: uncomment and implement
    }
}
