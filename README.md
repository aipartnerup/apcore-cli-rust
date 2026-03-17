# apcore-cli

[![crates.io](https://img.shields.io/crates/v/apcore-cli.svg)](https://crates.io/crates/apcore-cli)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Command-line interface for [apcore](https://apcore.aipartnerup.com/) modules.
Rust port of [apcore-cli-python](https://github.com/aipartnerup/apcore-cli-python) v0.2.0.

## Installation

```bash
cargo install apcore-cli
```

## Quick Start

```bash
# List all registered modules
apcore-cli list --format json

# Describe a module's schema
apcore-cli describe math.add

# Execute a module
apcore-cli math.add --a 3 --b 4

# Pipe JSON input via stdin
echo '{"a": 10, "b": 20}' | apcore-cli math.add --input -

# Shell completion
apcore-cli completion bash >> ~/.bashrc
```

## API Overview

| Public item | Description |
|---|---|
| `create_cli(extensions_dir, prog_name)` | Build the top-level clap Command |
| `build_module_command(module_def, executor)` | Build a subcommand from a ModuleDescriptor |
| `collect_input(stdin_flag, cli_kwargs, large_input)` | Merge CLI args + STDIN JSON |
| `validate_module_id(id)` | Validate a module identifier string |
| `ConfigResolver::resolve(key, cli_flag, env_var)` | 4-tier config resolution |
| `register_discovery_commands(cli, registry)` | Attach list + describe subcommands |
| `format_module_list / format_module_detail / format_exec_result` | TTY-adaptive output |
| `check_approval(module_id, auto_approve)` | HITL approval gate |
| `resolve_refs(schema, max_depth, module_id)` | JSON Schema $ref inliner |
| `schema_to_clap_args(schema)` | JSON Schema → clap Args |
| `register_shell_commands(cli, prog_name)` | Attach completion + man subcommands |
| `AuditLogger / AuthProvider / ConfigEncryptor / Sandbox` | Security layer |

## Exit Codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Module execution error / timeout |
| 2 | Invalid CLI input |
| 44 | Module not found / disabled |
| 45 | Schema validation error |
| 46 | Approval denied / no TTY |
| 47 | Config not found / invalid |
| 48 | Circular $ref / flag collision |
| 77 | ACL denied |
| 130 | SIGINT |

## Documentation

Full API reference: <https://apcore.aipartnerup.com/>
Protocol spec: `apcore/PROTOCOL_SPEC.md`

## License

Apache-2.0 — see [LICENSE](LICENSE).
