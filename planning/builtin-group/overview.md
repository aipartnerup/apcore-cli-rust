# FE-13 Built-in Command Group (`apcli`) — Rust Implementation

Port of the TypeScript reference (`../apcore-cli-typescript/src/builtin-group.ts`
+ dispatcher in `main.ts`) to Rust. Bumps apcore to 0.19.0 and
apcore-toolkit to 0.5.0.

## Task Execution Order

| ID | Task | Blocks |
|----|------|--------|
| T01 | Bump apcore 0.19.0 + toolkit 0.5.0 | — |
| T02 | Create `src/builtin_group.rs` (ApcliGroup + tests) | T07 |
| T03 | Add `ConfigResolver::resolve_object` | T07 |
| T04 | Split `discovery.rs` registrars | T07 |
| T05 | Split `system_cmd` / `shell` / `strategy` / `validate` / `init_cmd` registrars | T07 |
| T06 | Retire `BUILTIN_COMMANDS` → `RESERVED_GROUP_NAMES` | T07 |
| T07 | Integrate apcli group into `main.rs` + `CliConfig` | T08 |
| T08 | Integration tests (T-APCLI-01..41 parity) | T09 |
| T09 | `make check` / clippy / fmt | — |

## Key Design Decisions (spec §4.14 Rust normative)

```rust
pub struct ApcliConfig {
    pub mode: ApcliMode,
    pub disable_env: bool,
}
pub enum ApcliMode {
    Auto,                          // internal sentinel
    All,
    None,
    Include(Vec<String>),
    Exclude(Vec<String>),
}
```

`ApcliGroup` wraps the above with `registry_injected` + `from_cli_config`
flags to encode the 4-tier precedence (Tier 1 CliConfig > Tier 2 env >
Tier 3 yaml > Tier 4 auto-detect).
