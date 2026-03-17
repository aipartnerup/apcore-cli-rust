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
// Local trait abstractions for Registry and Executor
// ---------------------------------------------------------------------------
// apcore::Registry and apcore::Executor are concrete structs, not traits.
// These local traits allow LazyModuleGroup to be generic over both the real
// implementations and test mocks without depending on apcore internals.

/// Minimal registry interface required by LazyModuleGroup.
pub trait ModuleRegistry: Send + Sync {
    /// Return the names of all registered modules.
    fn list_modules(&self) -> Result<Vec<String>, String>;
    /// Return the descriptor for a module by name, or None if not found.
    fn get_module_descriptor(&self, name: &str) -> Option<apcore::registry::registry::ModuleDescriptor>;
}

/// Minimal executor interface required by LazyModuleGroup.
pub trait ModuleExecutor: Send + Sync {}

/// Adapter that implements ModuleRegistry for the real apcore::Registry.
pub struct ApCoreRegistryAdapter(pub apcore::Registry);

impl ModuleRegistry for ApCoreRegistryAdapter {
    fn list_modules(&self) -> Result<Vec<String>, String> {
        Ok(self.0.list(None, None).iter().map(|s| s.to_string()).collect())
    }

    fn get_module_descriptor(&self, name: &str) -> Option<apcore::registry::registry::ModuleDescriptor> {
        self.0.get_definition(name).cloned()
    }
}

/// Adapter that implements ModuleExecutor for the real apcore::Executor.
pub struct ApCoreExecutorAdapter(pub apcore::Executor);

impl ModuleExecutor for ApCoreExecutorAdapter {}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced by CLI dispatch operations.
#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid module id: {0}")]
    InvalidModuleId(String),

    #[error("reserved module id: '{0}' conflicts with a built-in command name")]
    ReservedModuleId(String),

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

/// Built-in command names that are always present regardless of the registry.
pub const BUILTIN_COMMANDS: &[&str] = &["completion", "describe", "exec", "list", "man"];

/// Lazy command registry: builds module subcommands on-demand from the
/// apcore Registry, caching them after first construction.
///
/// This is the Rust equivalent of the Python `LazyModuleGroup` (Click group
/// subclass with lazy `get_command` / `list_commands`).
pub struct LazyModuleGroup {
    registry: Arc<dyn ModuleRegistry>,
    executor: Arc<dyn ModuleExecutor>,
    /// Cache of module name -> name string (we store the name, not the Command,
    /// since clap::Command is not Clone in all configurations).
    module_cache: HashMap<String, bool>,
    /// Count of registry descriptor lookups (test instrumentation only).
    #[cfg(test)]
    pub registry_lookup_count: usize,
}

impl LazyModuleGroup {
    /// Create a new lazy module group.
    ///
    /// # Arguments
    /// * `registry` — module registry (real or mock)
    /// * `executor` — module executor (real or mock)
    pub fn new(
        registry: Arc<dyn ModuleRegistry>,
        executor: Arc<dyn ModuleExecutor>,
    ) -> Self {
        Self {
            registry,
            executor,
            module_cache: HashMap::new(),
            #[cfg(test)]
            registry_lookup_count: 0,
        }
    }

    /// Return sorted list of all command names: built-ins + module ids.
    pub fn list_commands(&self) -> Vec<String> {
        let mut names: Vec<String> = BUILTIN_COMMANDS.iter().map(|s| s.to_string()).collect();
        match self.registry.list_modules() {
            Ok(module_ids) => names.extend(module_ids),
            Err(e) => {
                tracing::warn!("Failed to list modules from registry: {e}");
            }
        }
        // Sort and dedup in one pass.
        names.sort_unstable();
        names.dedup();
        names
    }

    /// Look up a command by name. Returns `None` if the name is not a builtin
    /// and is not found in the registry.
    ///
    /// For module commands, builds and caches a lightweight clap Command.
    pub fn get_command(&mut self, name: &str) -> Option<clap::Command> {
        if BUILTIN_COMMANDS.contains(&name) {
            return Some(clap::Command::new(name.to_string()));
        }
        // Check the in-memory cache first.
        if self.module_cache.contains_key(name) {
            return Some(clap::Command::new(name.to_string()));
        }
        // Registry lookup.
        #[cfg(test)]
        {
            self.registry_lookup_count += 1;
        }
        let _descriptor = self.registry.get_module_descriptor(name)?;
        let cmd = clap::Command::new(name.to_string());
        self.module_cache.insert(name.to_string(), true);
        tracing::debug!("Loaded module command: {name}");
        Some(cmd)
    }

    /// Return the number of times the registry was queried for a descriptor.
    /// Available in test builds only.
    #[cfg(test)]
    pub fn registry_lookup_count(&self) -> usize {
        self.registry_lookup_count
    }
}

// ---------------------------------------------------------------------------
// build_module_command
// ---------------------------------------------------------------------------

/// Module IDs that are reserved for built-in commands. Passing one of these
/// as the module name to `build_module_command` returns a
/// `CliError::ReservedModuleId` error.
///
/// Must stay in sync with `BUILTIN_COMMANDS`.
pub const RESERVED_COMMAND_NAMES: &[&str] = &["completion", "describe", "exec", "list", "man"];

/// Built-in flag names added to every generated module command. A schema
/// property that collides with one of these names will cause
/// `std::process::exit(2)`.
const RESERVED_FLAG_NAMES: &[&str] = &["input", "yes", "large-input", "format", "sandbox"];

/// Build a clap `Command` for a single module definition.
///
/// The resulting subcommand has:
/// * its `name` set to `module_def.name`
/// * its `about` derived from the module descriptor (empty if unavailable)
/// * the 5 built-in flags: `--input`, `--yes`/`-y`, `--large-input`,
///   `--format`, `--sandbox`
/// * schema-derived flags from `schema_to_clap_args` (stub: empty vec)
///
/// `executor` is accepted for API symmetry with the Python counterpart but is
/// not embedded in the `clap::Command` (clap has no user-data attachment).
/// The executor is passed separately to the dispatch callback.
///
/// # Errors
/// Returns `CliError::ReservedModuleId` when `module_def.name` is one of the
/// reserved built-in command names.
pub fn build_module_command(
    module_def: &apcore::registry::registry::ModuleDescriptor,
    executor: Arc<dyn ModuleExecutor>,
) -> Result<clap::Command, CliError> {
    let module_id = &module_def.name;

    // Guard: reject reserved command names immediately.
    if RESERVED_COMMAND_NAMES.contains(&module_id.as_str()) {
        return Err(CliError::ReservedModuleId(module_id.clone()));
    }

    // TODO: resolve_refs is a stub — returns schema as-is for now.
    // When FE-08 is implemented, replace with:
    //   let resolved = crate::ref_resolver::resolve_refs(&mut schema, 32, module_id)?;
    let resolved_schema = module_def.input_schema.clone();

    // TODO: schema_to_clap_args is a stub — returns empty vec for now.
    // When FE-09 is implemented this will yield schema-derived flags.
    let schema_args = crate::schema_parser::schema_to_clap_args_or_empty(&resolved_schema);

    // Check for schema property names that collide with built-in flags.
    for arg in &schema_args {
        if let Some(long) = arg.get_long() {
            if RESERVED_FLAG_NAMES.contains(&long) {
                eprintln!(
                    "Error: module '{module_id}' schema property '{long}' conflicts \
                     with a reserved CLI option name. Rename the property."
                );
                std::process::exit(2);
            }
        }
    }

    // Suppress unused-variable warning; executor is kept for API symmetry.
    let _ = executor;

    let mut cmd = clap::Command::new(module_id.clone())
        // Built-in flags present on every generated command.
        .arg(
            clap::Arg::new("input")
                .long("input")
                .value_name("SOURCE")
                .help("Read input from file or STDIN ('-')."),
        )
        .arg(
            clap::Arg::new("yes")
                .long("yes")
                .short('y')
                .action(clap::ArgAction::SetTrue)
                .help("Bypass approval prompts."),
        )
        .arg(
            clap::Arg::new("large-input")
                .long("large-input")
                .action(clap::ArgAction::SetTrue)
                .help("Allow STDIN input larger than 10MB."),
        )
        .arg(
            clap::Arg::new("format")
                .long("format")
                .value_parser(["json", "table"])
                .help("Output format."),
        )
        .arg(
            clap::Arg::new("sandbox")
                .long("sandbox")
                .action(clap::ArgAction::SetTrue)
                .help("Run module in subprocess sandbox."),
        );

    // Attach schema-derived args (stub: none for now).
    for arg in schema_args {
        cmd = cmd.arg(arg);
    }

    Ok(cmd)
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

    // ---------------------------------------------------------------------------
    // build_module_command tests (TDD — RED written before GREEN)
    // ---------------------------------------------------------------------------

    /// Construct a minimal `ModuleDescriptor` for use in `build_module_command`
    /// tests. `input_schema` defaults to a JSON null (no properties) when
    /// `schema` is `None`.
    fn make_module_descriptor(
        name: &str,
        _description: &str,
        schema: Option<serde_json::Value>,
    ) -> apcore::registry::registry::ModuleDescriptor {
        apcore::registry::registry::ModuleDescriptor {
            name: name.to_string(),
            annotations: apcore::module::ModuleAnnotations::default(),
            input_schema: schema.unwrap_or(serde_json::Value::Null),
            output_schema: serde_json::Value::Object(Default::default()),
            enabled: true,
            tags: vec![],
            dependencies: vec![],
        }
    }

    #[test]
    fn test_build_module_command_name_is_set() {
        let module = make_module_descriptor("math.add", "Add two numbers", None);
        let executor = mock_executor();
        let cmd = build_module_command(&module, executor).unwrap();
        assert_eq!(cmd.get_name(), "math.add");
    }

    #[test]
    fn test_build_module_command_has_input_flag() {
        let module = make_module_descriptor("a.b", "desc", None);
        let executor = mock_executor();
        let cmd = build_module_command(&module, executor).unwrap();
        let names: Vec<&str> = cmd.get_opts().filter_map(|a| a.get_long()).collect();
        assert!(names.contains(&"input"), "must have --input flag");
    }

    #[test]
    fn test_build_module_command_has_yes_flag() {
        let module = make_module_descriptor("a.b", "desc", None);
        let executor = mock_executor();
        let cmd = build_module_command(&module, executor).unwrap();
        let names: Vec<&str> = cmd.get_opts().filter_map(|a| a.get_long()).collect();
        assert!(names.contains(&"yes"), "must have --yes flag");
    }

    #[test]
    fn test_build_module_command_has_large_input_flag() {
        let module = make_module_descriptor("a.b", "desc", None);
        let executor = mock_executor();
        let cmd = build_module_command(&module, executor).unwrap();
        let names: Vec<&str> = cmd.get_opts().filter_map(|a| a.get_long()).collect();
        assert!(names.contains(&"large-input"), "must have --large-input flag");
    }

    #[test]
    fn test_build_module_command_has_format_flag() {
        let module = make_module_descriptor("a.b", "desc", None);
        let executor = mock_executor();
        let cmd = build_module_command(&module, executor).unwrap();
        let names: Vec<&str> = cmd.get_opts().filter_map(|a| a.get_long()).collect();
        assert!(names.contains(&"format"), "must have --format flag");
    }

    #[test]
    fn test_build_module_command_has_sandbox_flag() {
        let module = make_module_descriptor("a.b", "desc", None);
        let executor = mock_executor();
        let cmd = build_module_command(&module, executor).unwrap();
        let names: Vec<&str> = cmd.get_opts().filter_map(|a| a.get_long()).collect();
        assert!(names.contains(&"sandbox"), "must have --sandbox flag");
    }

    #[test]
    fn test_build_module_command_reserved_name_returns_error() {
        for reserved in RESERVED_COMMAND_NAMES {
            let module = make_module_descriptor(reserved, "desc", None);
            let executor = mock_executor();
            let result = build_module_command(&module, executor);
            assert!(
                matches!(result, Err(CliError::ReservedModuleId(_))),
                "expected ReservedModuleId for '{reserved}', got {result:?}"
            );
        }
    }

    #[test]
    fn test_build_module_command_yes_has_short_flag() {
        let module = make_module_descriptor("a.b", "desc", None);
        let executor = mock_executor();
        let cmd = build_module_command(&module, executor).unwrap();
        let has_short_y = cmd
            .get_opts()
            .filter(|a| a.get_long() == Some("yes"))
            .any(|a| a.get_short() == Some('y'));
        assert!(has_short_y, "--yes must have short flag -y");
    }

    // ---------------------------------------------------------------------------
    // LazyModuleGroup tests (TDD)
    // ---------------------------------------------------------------------------

    /// Mock registry that returns a fixed list of module names.
    struct MockRegistry {
        modules: Vec<String>,
    }

    impl ModuleRegistry for MockRegistry {
        fn list_modules(&self) -> Result<Vec<String>, String> {
            Ok(self.modules.clone())
        }

        fn get_module_descriptor(
            &self,
            name: &str,
        ) -> Option<apcore::registry::registry::ModuleDescriptor> {
            if self.modules.iter().any(|m| m == name) {
                Some(apcore::registry::registry::ModuleDescriptor {
                    name: name.to_string(),
                    annotations: apcore::module::ModuleAnnotations::default(),
                    input_schema: serde_json::Value::Object(Default::default()),
                    output_schema: serde_json::Value::Object(Default::default()),
                    enabled: true,
                    tags: vec![],
                    dependencies: vec![],
                })
            } else {
                None
            }
        }
    }

    /// Mock registry whose list_modules() always returns an error.
    struct ErrorRegistry;

    impl ModuleRegistry for ErrorRegistry {
        fn list_modules(&self) -> Result<Vec<String>, String> {
            Err("registry unavailable".to_string())
        }

        fn get_module_descriptor(
            &self,
            _name: &str,
        ) -> Option<apcore::registry::registry::ModuleDescriptor> {
            None
        }
    }

    /// Mock executor (no-op).
    struct MockExecutor;

    impl ModuleExecutor for MockExecutor {}

    fn mock_registry(modules: Vec<&str>) -> Arc<dyn ModuleRegistry> {
        Arc::new(MockRegistry {
            modules: modules.iter().map(|s| s.to_string()).collect(),
        })
    }

    fn mock_executor() -> Arc<dyn ModuleExecutor> {
        Arc::new(MockExecutor)
    }

    #[test]
    fn test_lazy_module_group_list_commands_empty_registry() {
        let group = LazyModuleGroup::new(mock_registry(vec![]), mock_executor());
        let cmds = group.list_commands();
        for builtin in ["exec", "list", "describe", "completion", "man"] {
            assert!(
                cmds.contains(&builtin.to_string()),
                "missing builtin: {builtin}"
            );
        }
        // Result must be sorted.
        let mut sorted = cmds.clone();
        sorted.sort();
        assert_eq!(cmds, sorted, "list_commands must return a sorted list");
    }

    #[test]
    fn test_lazy_module_group_list_commands_includes_modules() {
        let group =
            LazyModuleGroup::new(mock_registry(vec!["math.add", "text.summarize"]), mock_executor());
        let cmds = group.list_commands();
        assert!(cmds.contains(&"math.add".to_string()));
        assert!(cmds.contains(&"text.summarize".to_string()));
    }

    #[test]
    fn test_lazy_module_group_list_commands_registry_error() {
        let group = LazyModuleGroup::new(Arc::new(ErrorRegistry), mock_executor());
        let cmds = group.list_commands();
        // Must not be empty; must contain builtins.
        assert!(!cmds.is_empty());
        assert!(cmds.contains(&"list".to_string()));
    }

    #[test]
    fn test_lazy_module_group_get_command_builtin() {
        let mut group = LazyModuleGroup::new(mock_registry(vec![]), mock_executor());
        let cmd = group.get_command("list");
        assert!(cmd.is_some(), "get_command('list') must return Some");
    }

    #[test]
    fn test_lazy_module_group_get_command_not_found() {
        let mut group = LazyModuleGroup::new(mock_registry(vec![]), mock_executor());
        let cmd = group.get_command("nonexistent.module");
        assert!(cmd.is_none());
    }

    #[test]
    fn test_lazy_module_group_get_command_caches_module() {
        let mut group =
            LazyModuleGroup::new(mock_registry(vec!["math.add"]), mock_executor());
        // First call builds and caches.
        let cmd1 = group.get_command("math.add");
        assert!(cmd1.is_some());
        // Second call returns from cache — registry lookup should not be called again.
        let cmd2 = group.get_command("math.add");
        assert!(cmd2.is_some());
        assert_eq!(group.registry_lookup_count(), 1, "cached after first lookup");
    }

    #[test]
    fn test_lazy_module_group_builtin_commands_sorted() {
        // BUILTIN_COMMANDS slice must itself be in sorted order (single source of truth).
        let mut sorted = BUILTIN_COMMANDS.to_vec();
        sorted.sort_unstable();
        assert_eq!(
            BUILTIN_COMMANDS, sorted.as_slice(),
            "BUILTIN_COMMANDS must be sorted"
        );
    }

    #[test]
    fn test_lazy_module_group_list_deduplicates_builtins() {
        // If a registry module name collides with a builtin, the result must not
        // contain duplicates.
        let group = LazyModuleGroup::new(mock_registry(vec!["list", "exec"]), mock_executor());
        let cmds = group.list_commands();
        let list_count = cmds.iter().filter(|c| c.as_str() == "list").count();
        assert_eq!(list_count, 1, "duplicate 'list' entry in list_commands");
    }
}
