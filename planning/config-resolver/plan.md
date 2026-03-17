# Implementation Plan: Config Resolver (Rust)

**Feature ID**: FE-07
**Status**: planned
**Priority**: P0
**Source Spec**: apcore-cli/docs/features/config-resolver.md
**Target Module**: `src/config.rs`

---

## Goal

Implement a fully-functional `ConfigResolver` that resolves configuration values via 4-tier precedence (CLI flag > environment variable > YAML config file > built-in default), replacing the current `todo!()` stubs in `src/config.rs` and its integration test counterpart `tests/test_config.rs`.

---

## Architecture Design

### Component Structure

The feature lives entirely in the existing `src/config.rs` module, which is already declared as `pub mod config` in `src/lib.rs` and re-exported as `pub use config::ConfigResolver`. No new files or modules need to be created; the plan fills in three stub methods and completes the integration test file.

```
src/config.rs
├── ConfigResolver (struct)          ← already scaffolded
│   ├── DEFAULTS: &[(&str, &str)]   ← already correct
│   ├── new()                        ← complete (calls load_config_file)
│   ├── resolve()                    ← STUB — implement tier 1-4 logic
│   ├── load_config_file()           ← STUB — read YAML, flatten, warn
│   └── flatten_dict()               ← STUB — recursive dot-notation flatten
tests/test_config.rs                 ← integration tests — assert!() stubs
```

### Data Flow

```
ConfigResolver::new(cli_flags, config_path)
  └─► load_config_file(path)
        ├─ not found          → None (silent)
        ├─ malformed YAML     → None + warn!()
        ├─ non-dict root      → None + warn!()
        └─ valid dict         → flatten_dict() → HashMap<String, String>

ConfigResolver::resolve(key, cli_flag, env_var)
  ├─ Tier 1: cli_flags.get(cli_flag) == Some(Some(value))  → return value
  ├─ Tier 2: std::env::var(env_var)  != "" && != Err       → return value
  ├─ Tier 3: config_file.get(key)    is Some(value)         → return value
  └─ Tier 4: defaults.get(key)       is Some(value)         → return Some(value)
             (unknown key)                                   → return None
```

### Technology Choices with Rationale

| Concern | Choice | Rationale |
|---|---|---|
| YAML parsing | `serde_yaml 0.9` (already in `Cargo.toml`) | Matches spec; `serde_yaml::Value` natively represents nested maps |
| Recursive flattening | `serde_yaml::Value::Mapping` traversal | Avoids adding `serde_json` for YAML; keeps the flatten logic self-contained |
| Warning emission | `tracing::warn!()` (already imported) | Consistent with rest of codebase; mirrors Python `logging.warning()` |
| Type for resolved values | `Option<String>` | All config values are strings at resolution time; callers parse further if needed |
| CLI flags map | `HashMap<String, Option<String>>` | `None` entry means flag was registered but not provided — intentional fall-through |
| Env var empty-string rule | explicit `env_value.is_empty()` guard | Non-obvious deliberate choice documented in feature spec lessons |

---

## Task Breakdown

### Dependency Graph

```mermaid
graph LR
    models["models<br/>(defaults + types)"]
    resolver["resolver<br/>(resolve + load + flatten)"]
    tests["tests<br/>(integration assertions)"]

    models --> resolver
    resolver --> tests
```

### Task List

| Task ID | Title | Estimated Time | Depends On |
|---|---|---|---|
| `models` | Audit and finalise defaults, types, and public API surface | ~30 min | — |
| `resolver` | Implement `resolve`, `load_config_file`, and `flatten_dict` | ~1.5 h | `models` |
| `tests` | Complete integration test assertions in `tests/test_config.rs` | ~1 h | `resolver` |

**Total estimated time**: ~3 hours

---

## Risks and Considerations

### 1. `serde_yaml::Value` vs `serde_json::Value` in `flatten_dict`

The current stub signature accepts `serde_json::Value`. The YAML loader returns `serde_yaml::Value`. These are distinct types. The implementation must decide whether to:
- Accept `serde_yaml::Value` internally and convert at the boundary, or
- Convert once with `serde_yaml::to_value` / `serde_json` bridge.

**Decision**: Change the internal `flatten_dict` helper to accept `serde_yaml::Value` (a private implementation detail). The public method signature can be kept for JSON-facing callers or removed if not externally used. The `load_config_file` path should flatten directly from the YAML value.

### 2. Thread Safety of `std::env::set_var` in Tests

Rust integration tests run in separate processes (unlike unit tests which share a process). However, tests within the same binary may run in parallel. Setting environment variables in integration tests is `unsafe` in Rust 1.81+ and can cause data races across parallel tests.

**Mitigation**: Use `std::env::set_var` only with careful test isolation. The existing `strip_apcore_env_vars()` helper in `tests/common/mod.rs` provides cleanup. Tests that set env vars should use a serial execution guard or rely on unique env var names. Document this clearly in the test file.

### 3. YAML Non-Dict Root

A YAML file that parses to a scalar or a sequence (e.g., `- item`) is valid YAML but not a valid config root. `serde_yaml::Value::Mapping` match will fail; the code must handle this as a WARNING + return None, identical to malformed YAML behavior per FR-DISP-005 AF-2.

### 4. `cli.auto_approve` in DEFAULTS

The Python implementation includes `cli.auto_approve: False` in DEFAULTS (visible in `config.py` line 25) but the current Rust DEFAULTS constant does not include it. The `models` task must add this to match the spec's configuration key table (Section 5 of the feature spec).

### 5. `flatten_dict` Public Method Signature

The current stub exposes `flatten_dict` as a `pub` method taking `serde_json::Value`. If this is part of the public API contract, changing its type to `serde_yaml::Value` would be a breaking change. Keep the `serde_json::Value` variant for the public method but implement the internal recursive flatten on `serde_yaml::Value`.

---

## Acceptance Criteria

- [ ] `cargo test` passes with zero failures across all test targets
- [ ] `ConfigResolver::resolve` returns the CLI flag value when set and non-None (T-CFG-01)
- [ ] `ConfigResolver::resolve` falls through to env var when CLI flag absent (T-CFG-02)
- [ ] `ConfigResolver::resolve` falls through to config file when env var absent (T-CFG-03)
- [ ] `ConfigResolver::resolve` returns built-in default when no tier matches (T-CFG-04)
- [ ] Missing config file is silently ignored — no panic (T-CFG-05)
- [ ] Malformed YAML config file emits a `warn!()` and returns None — no panic (T-CFG-06)
- [ ] Nested YAML keys are flattened to dot-notation (`extensions.root`) (T-CFG-07)
- [ ] Empty-string env var is treated as absent and falls through (T-CFG-08)
- [ ] CLI flag value of `None` (flag registered but not provided) falls through (T-CFG-09)
- [ ] `DEFAULTS` contains all five keys from the spec: `extensions.root`, `logging.level`, `sandbox.enabled`, `cli.stdin_buffer_limit`, `cli.auto_approve`
- [ ] No `todo!()` macros remain in `src/config.rs`
- [ ] All `assert!(false, "not implemented")` stubs in `tests/test_config.rs` are replaced with real assertions

---

## References

- Feature spec: `apcore-cli/docs/features/config-resolver.md` (FE-07)
- Python reference implementation: `apcore-cli-python/src/apcore_cli/config.py`
- Python planning doc: `apcore-cli-python/planning/config-resolver.md`
- Type mapping spec: `apcore/docs/spec/type-mapping.md`
- Rust target: `apcore-cli-rust/src/config.rs`
- Integration tests: `apcore-cli-rust/tests/test_config.rs`
- Shared test helpers: `apcore-cli-rust/tests/common/mod.rs`
