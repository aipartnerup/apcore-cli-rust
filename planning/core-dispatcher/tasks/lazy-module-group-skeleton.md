# Task: lazy-module-group-skeleton

**Feature**: FE-01 Core Dispatcher
**File**: `src/cli.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~3h
**Depends on**: `validate-module-id`, `collect-input`
**Required by**: `build-module-command`, `create-cli-and-main`

---

## Context

Python's `LazyModuleGroup` is a `click.Group` subclass with `list_commands` and `get_command` hooks. Clap v4 has no equivalent extension point. In Rust, `LazyModuleGroup` becomes a plain struct that:

1. Holds shared references to the registry and executor.
2. Provides `list_commands() -> Vec<String>` for help text and shell completion.
3. Provides `get_command(name: &str) -> Option<clap::Command>` with an in-memory cache.

The struct is used by `create_cli` to assemble the help output and by the dispatch loop in `main` for runtime command resolution. It is not a clap extension.

The current stub in `cli.rs` has empty fields and `todo!` bodies. This task fills them in.

---

## RED — Write Failing Tests First

Add to `tests/test_cli.rs`:

```rust
use apcore_cli::cli::LazyModuleGroup;
// Assume a test helper that creates a mock Registry + Executor:
use common::{mock_registry_with_modules, mock_executor};

#[test]
fn test_lazy_module_group_list_commands_empty_registry() {
    let registry = mock_registry_with_modules(vec![]);
    let executor = mock_executor();
    let group = LazyModuleGroup::new(Arc::new(registry), Arc::new(executor));
    let cmds = group.list_commands();
    // Must contain all builtins.
    for builtin in ["exec", "list", "describe", "completion", "man"] {
        assert!(cmds.contains(&builtin.to_string()), "missing builtin: {builtin}");
    }
    // Result must be sorted.
    let mut sorted = cmds.clone();
    sorted.sort();
    assert_eq!(cmds, sorted, "list_commands must return sorted list");
}

#[test]
fn test_lazy_module_group_list_commands_includes_modules() {
    let registry = mock_registry_with_modules(vec!["math.add", "text.summarize"]);
    let executor = mock_executor();
    let group = LazyModuleGroup::new(Arc::new(registry), Arc::new(executor));
    let cmds = group.list_commands();
    assert!(cmds.contains(&"math.add".to_string()));
    assert!(cmds.contains(&"text.summarize".to_string()));
}

#[test]
fn test_lazy_module_group_list_commands_registry_error() {
    // Registry that panics on list() — group must catch and return only builtins.
    let registry = mock_registry_list_error();
    let executor = mock_executor();
    let group = LazyModuleGroup::new(Arc::new(registry), Arc::new(executor));
    let cmds = group.list_commands();
    // Must not be empty; must contain builtins.
    assert!(cmds.contains(&"list".to_string()));
}

#[test]
fn test_lazy_module_group_get_command_builtin() {
    let registry = mock_registry_with_modules(vec![]);
    let executor = mock_executor();
    let mut group = LazyModuleGroup::new(Arc::new(registry), Arc::new(executor));
    // Built-in commands are pre-registered; get_command returns Some for each.
    let cmd = group.get_command("list");
    assert!(cmd.is_some(), "get_command('list') must return Some");
}

#[test]
fn test_lazy_module_group_get_command_not_found() {
    let registry = mock_registry_with_modules(vec![]);
    let executor = mock_executor();
    let mut group = LazyModuleGroup::new(Arc::new(registry), Arc::new(executor));
    let cmd = group.get_command("nonexistent.module");
    assert!(cmd.is_none());
}

#[test]
fn test_lazy_module_group_get_command_caches_module() {
    let registry = mock_registry_with_modules(vec!["math.add"]);
    let executor = mock_executor();
    let mut group = LazyModuleGroup::new(Arc::new(registry), Arc::new(executor));
    // First call builds and caches.
    let cmd1 = group.get_command("math.add");
    assert!(cmd1.is_some());
    // Second call returns from cache — registry.get_definition should not be called again.
    // (Verify via call counter on the mock registry.)
    let cmd2 = group.get_command("math.add");
    assert!(cmd2.is_some());
    assert_eq!(group.registry_lookup_count(), 1, "cached after first lookup");
}
```

Add a `tests/common/mod.rs` helper with `mock_registry_with_modules`, `mock_executor`, `mock_registry_list_error`. These return structs implementing the apcore `Registry` and `Executor` traits.

Run `cargo test lazy_module_group` — all fail.

---

## GREEN — Implement

Update `LazyModuleGroup` in `src/cli.rs`:

```rust
use std::collections::HashMap;
use std::sync::Arc;
use apcore::{Executor, Registry};

const BUILTIN_COMMANDS: &[&str] = &["exec", "list", "describe", "completion", "man"];

pub struct LazyModuleGroup {
    registry: Arc<dyn Registry + Send + Sync>,
    executor: Arc<dyn Executor + Send + Sync>,
    // Cache of module name → built clap::Command.
    // Commands are not Clone, so we store them by name and rebuild on demand
    // for the help path; the execution path uses the executor directly.
    module_cache: HashMap<String, clap::Command>,
    // For testability: count of registry.get_definition calls.
    #[cfg(test)]
    pub registry_lookup_count: usize,
}

impl LazyModuleGroup {
    pub fn new(
        registry: Arc<dyn Registry + Send + Sync>,
        executor: Arc<dyn Executor + Send + Sync>,
    ) -> Self {
        Self {
            registry,
            executor,
            module_cache: HashMap::new(),
            #[cfg(test)]
            registry_lookup_count: 0,
        }
    }

    /// Return sorted list of all command names: built-ins + module IDs.
    pub fn list_commands(&self) -> Vec<String> {
        let mut names: Vec<String> = BUILTIN_COMMANDS.iter().map(|s| s.to_string()).collect();
        match self.registry.list() {
            Ok(module_ids) => names.extend(module_ids),
            Err(e) => {
                tracing::warn!("Failed to list modules from registry: {e}");
            }
        }
        // Dedup and sort.
        let mut unique: Vec<String> = names.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect();
        unique.sort();
        unique
    }

    /// Look up a command by name. Returns None if not found in builtins or registry.
    /// For module commands, builds and caches the clap::Command.
    pub fn get_command(&mut self, name: &str) -> Option<clap::Command> {
        // Built-ins are resolved by the clap subcommand tree, not this cache.
        // Return a sentinel Some only for built-ins so callers can check membership.
        if BUILTIN_COMMANDS.contains(&name) {
            return Some(clap::Command::new(name)); // lightweight placeholder
        }
        // Check cache.
        if self.module_cache.contains_key(name) {
            return self.module_cache.get(name).cloned();
        }
        // Registry lookup.
        #[cfg(test)]
        { self.registry_lookup_count += 1; }
        let module_def = self.registry.get_definition(name).ok().flatten()?;
        let cmd = build_module_command(&module_def, Arc::clone(&self.executor));
        self.module_cache.insert(name.to_string(), cmd.clone());
        Some(cmd)
    }

    #[cfg(test)]
    pub fn registry_lookup_count(&self) -> usize {
        self.registry_lookup_count
    }
}
```

Note: `clap::Command` does not implement `Clone` in all versions. If `Clone` is unavailable, the cache stores the command by name and `get_command` returns a freshly built command each time (cache is used only to avoid repeated registry calls, not to share the `Command` object). Adjust accordingly.

---

## REFACTOR

- Verify `BUILTIN_COMMANDS` slice is the single source of truth — remove any duplicate definition.
- Replace `std::collections::HashSet` dedup with `sort_unstable` + `dedup` on the vec for efficiency.
- Add `tracing::debug!("Loaded module command: {name}")` after cache insert.
- Run `cargo clippy -- -D warnings`.

---

## Verification

```bash
cargo test lazy_module_group 2>&1
# Expected: test result: ok. N passed; 0 failed
```

List-commands behaviour is also exercised by `T-DISP-01` in `tests/test_e2e.rs`.
