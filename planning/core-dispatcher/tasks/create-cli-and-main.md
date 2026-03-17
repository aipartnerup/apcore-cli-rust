# Task: create-cli-and-main

**Feature**: FE-01 Core Dispatcher
**File**: `src/main.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~4h
**Depends on**: `exec-dispatch-callback`
**Required by**: nothing (final integration task)

---

## Context

This task implements the binary entry point: `extract_extensions_dir`, `create_cli`, and `main`. It is the Rust equivalent of `apcore_cli/__main__.py`. The goal is to wire all previously implemented components (registry, executor, audit logger, `LazyModuleGroup`, built-in commands, dynamic dispatch) into a single `async fn main`.

Key structural decisions:

- `create_cli` returns `clap::Command` (the root command tree with built-in subcommands). Dynamic module dispatch is handled in `main` after `get_matches`, not inside the `Command` tree.
- `extract_extensions_dir` pre-parses `--extensions-dir` from `std::env::args()` before clap runs.
- Program name is resolved from `std::env::args().next()` basename, with `prog_name` parameter override (FR-01-06).
- Log level is resolved using three-tier precedence with `tracing-subscriber` + `EnvFilter`.
- Extensions directory validation exits 47 with exact message on missing or unreadable path.
- `allow_external_subcommands(true)` enables dynamic module dispatch for unrecognised subcommands.

---

## RED — Write Failing Tests First

Add to `tests/test_e2e.rs`:

```rust
#[test]
fn test_help_flag_exits_0_contains_builtins() {
    let out = run_apcore(&["--extensions-dir", "./tests/fixtures/extensions", "--help"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    for builtin in ["list", "describe", "completion"] {
        assert!(stdout.contains(builtin), "help must mention '{builtin}'");
    }
}

#[test]
fn test_version_flag_format() {
    let out = run_apcore(&["--version"]);
    assert_eq!(out.status.code(), Some(0));
    let output = String::from_utf8_lossy(&out.stdout);
    // Must match "apcore-cli, version X.Y.Z" per FR-01-04.
    assert!(
        output.contains("apcore-cli") && output.contains("version"),
        "version output: {output}"
    );
}

#[test]
fn test_extensions_dir_missing_exits_47() {
    let out = run_apcore(&["--extensions-dir", "/tmp/definitely_does_not_exist_apcore_test"]);
    assert_eq!(out.status.code(), Some(47));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Extensions directory not found") || stderr.contains("not found"));
}

#[test]
fn test_extensions_dir_env_var_respected() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_apcore-cli"))
        .env("APCORE_EXTENSIONS_ROOT", "./tests/fixtures/extensions")
        .args(&["--help"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn test_extensions_dir_flag_overrides_env() {
    // --extensions-dir flag takes precedence over APCORE_EXTENSIONS_ROOT.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_apcore-cli"))
        .env("APCORE_EXTENSIONS_ROOT", "/nonexistent/path")
        .args(&["--extensions-dir", "./tests/fixtures/extensions", "--help"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn test_prog_name_in_version_output() {
    // When invoked as "apcore-cli", version output must contain "apcore-cli".
    let out = run_apcore(&["--version"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("apcore-cli"), "stdout: {stdout}");
}
```

Run `cargo test --test test_e2e` — all fail (main is a stub).

---

## GREEN — Implement

Replace the entire `src/main.rs` with:

```rust
// apcore-cli — Binary entry point.
// Protocol spec: FE-01 (create_cli, main, extract_extensions_dir)

use std::path::Path;
use std::sync::Arc;

use apcore::{Executor, Registry};
use apcore_cli::{
    cli::{dispatch_module, set_audit_logger, LazyModuleGroup},
    config::ConfigResolver,
    discovery::register_discovery_commands,
    security::AuditLogger,
    shell::register_shell_commands,
    EXIT_CONFIG_NOT_FOUND,
};

/// Pre-parse `--extensions-dir` from raw argv before clap processes arguments.
///
/// Required because the registry must be instantiated before clap runs.
/// Mirrors Python's `_extract_extensions_dir`.
fn extract_extensions_dir(args: &[String]) -> Option<String> {
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--extensions-dir" {
            return iter.next().cloned();
        }
        if let Some(val) = arg.strip_prefix("--extensions-dir=") {
            return Some(val.to_string());
        }
    }
    None
}

/// Resolve the program name from argv[0] basename, with an explicit override.
fn resolve_prog_name(prog_name: Option<String>) -> String {
    if let Some(name) = prog_name {
        return name;
    }
    std::env::args()
        .next()
        .as_deref()
        .and_then(|s| Path::new(s).file_name()?.to_str())
        .unwrap_or("apcore-cli")
        .to_string()
}

/// Initialise tracing with three-tier log-level precedence:
/// APCORE_CLI_LOGGING_LEVEL > APCORE_LOGGING_LEVEL > WARNING.
fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    let cli_level = std::env::var("APCORE_CLI_LOGGING_LEVEL").unwrap_or_default();
    let global_level = std::env::var("APCORE_LOGGING_LEVEL").unwrap_or_default();
    let level_str = if !cli_level.is_empty() {
        cli_level
    } else if !global_level.is_empty() {
        global_level
    } else {
        "warn".to_string()
    };

    let filter = EnvFilter::try_new(&level_str)
        .unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

/// Build the root clap::Command tree.
///
/// The tree contains built-in subcommands only. Dynamic module dispatch is
/// handled in `main` via `allow_external_subcommands`.
pub fn create_cli(
    extensions_dir: Option<String>,
    prog_name: Option<String>,
) -> (clap::Command, Arc<dyn Registry + Send + Sync>, Arc<dyn Executor + Send + Sync>) {
    let name = resolve_prog_name(prog_name);

    // Resolve extensions_dir.
    let ext_dir = if let Some(dir) = extensions_dir {
        dir
    } else {
        let config = ConfigResolver::new(None, None);
        config
            .resolve("extensions.root", Some("--extensions-dir"), Some("APCORE_EXTENSIONS_ROOT"))
            .unwrap_or_else(|| "./extensions".to_string())
    };

    // Validate extensions directory.
    let path = Path::new(&ext_dir);
    if !path.exists() {
        eprintln!(
            "Error: Extensions directory not found: '{ext_dir}'. \
             Set APCORE_EXTENSIONS_ROOT or verify the path."
        );
        std::process::exit(EXIT_CONFIG_NOT_FOUND);
    }
    // Check readability (platform check via metadata).
    if std::fs::read_dir(path).is_err() {
        eprintln!(
            "Error: Cannot read extensions directory: '{ext_dir}'. Check file permissions."
        );
        std::process::exit(EXIT_CONFIG_NOT_FOUND);
    }

    // Instantiate registry and executor.
    tracing::debug!("Loading extensions from {ext_dir}");
    let registry = Arc::new(Registry::new(&ext_dir));
    match registry.discover() {
        Ok(count) => tracing::info!("Initialized {name} with {count} modules."),
        Err(e) => tracing::warn!("Discovery failed: {e}"),
    }
    let executor = Arc::new(Executor::new(Arc::clone(&registry)));

    // Initialise audit logger.
    match AuditLogger::new() {
        Ok(logger) => set_audit_logger(Some(logger)),
        Err(e) => tracing::warn!("Failed to initialise audit logger: {e}"),
    }

    // Build root command.
    let mut cmd = clap::Command::new(name.clone())
        .version(env!("CARGO_PKG_VERSION"))
        .about("CLI adapter for the apcore module ecosystem.")
        .allow_external_subcommands(true)
        .arg(
            clap::Arg::new("extensions-dir")
                .long("extensions-dir")
                .global(true)
                .value_name("PATH")
                .help("Path to apcore extensions directory."),
        )
        .arg(
            clap::Arg::new("log-level")
                .long("log-level")
                .global(true)
                .value_parser(["DEBUG", "INFO", "WARNING", "ERROR"])
                .ignore_case(true)
                .help("Log verbosity."),
        );

    // Register built-in subcommands from discovery and shell modules.
    let lazy_group = LazyModuleGroup::new(Arc::clone(&registry), Arc::clone(&executor));
    cmd = register_discovery_commands(cmd, Arc::clone(&registry));
    cmd = register_shell_commands(cmd, &name);

    (cmd, registry, executor)
}

#[tokio::main]
async fn main() {
    init_tracing();

    let raw_args: Vec<String> = std::env::args().collect();
    let extensions_dir = extract_extensions_dir(&raw_args[1..]);

    let (cmd, registry, executor) = create_cli(extensions_dir, None);
    let matches = cmd.get_matches();

    match matches.subcommand() {
        Some(("list", sub_m))       => apcore_cli::discovery::cmd_list(&registry, sub_m).await,
        Some(("describe", sub_m))   => apcore_cli::discovery::cmd_describe(&registry, sub_m).await,
        Some(("completion", sub_m)) => apcore_cli::shell::cmd_completion(sub_m),
        Some(("exec", sub_m)) => {
            let module_id = sub_m.get_one::<String>("MODULE_ID")
                .map(|s| s.as_str())
                .unwrap_or("");
            dispatch_module(module_id, sub_m, &registry, &executor).await
        }
        Some((external, sub_m)) => {
            // Dynamic module dispatch for unrecognised subcommands.
            dispatch_module(external, sub_m, &registry, &executor).await
        }
        None => {
            // No subcommand: print help.
            let _ = clap::Command::new("apcore-cli")
                .print_help();
            std::process::exit(0);
        }
    }
}
```

Note: `register_discovery_commands` and `register_shell_commands` must return `clap::Command` rather than mutating a `click.Group`. Update their signatures in `discovery.rs` and `shell.rs` to accept and return `clap::Command` if they don't already match this pattern.

---

## REFACTOR

- Extract `validate_extensions_dir(path: &str) -> Result<(), i32>` helper to avoid duplicated guard logic.
- Ensure `--log-level` flag at runtime reloads the tracing filter. Use `tracing_subscriber::reload` handle stored as a global `OnceLock<Handle>`.
- Confirm `clap::Command` version output format: clap 4 outputs `{name} {version}` by default. The spec requires `{name}, version {version}` (with comma). Override with `.version_template("{name}, version {version}")` or a custom `--version` handler.
- Run `cargo clippy -- -D warnings` and `cargo build --release`.

---

## Verification

```bash
# Full test suite:
cargo test 2>&1

# Manual smoke tests:
./target/debug/apcore-cli --version
# Expected: "apcore-cli, version 0.2.0"

./target/debug/apcore-cli --extensions-dir tests/fixtures/extensions --help
# Expected: exit 0, contains "list", "describe", "completion"

./target/debug/apcore-cli --extensions-dir /nonexistent
# Expected: exit 47, stderr contains "not found"

echo '{"a":5,"b":10}' | ./target/debug/apcore-cli \
    --extensions-dir tests/fixtures/extensions \
    exec math.add --input -
# Expected: exit 0, result on stdout

# Startup time benchmark (requires hyperfine):
hyperfine './target/release/apcore-cli --extensions-dir tests/fixtures/extensions --help'
# Expected: < 100 ms mean
```
