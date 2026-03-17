# Task: man-page-generator

**Feature**: FE-06 Shell Integration
**File**: `src/shell.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~3h
**Depends on**: `shell-error-type`
**Required by**: `cli-wiring`

---

## Context

This task implements the man page generation pipeline: `build_synopsis`, `generate_man_page`, and `cmd_man`. Together they produce a roff-formatted man page for any named CLI subcommand.

Key behaviours:

1. `build_synopsis(cmd_opt, prog_name, command_name)` constructs the `.SH SYNOPSIS` line by iterating over a clap `Command`'s arguments. Optional options get brackets; required positionals do not.
2. `generate_man_page(command_name, cmd_opt, prog_name, version)` assembles all roff sections in the order: `.TH`, `.SH NAME`, `.SH SYNOPSIS`, `.SH DESCRIPTION` (if present), `.SH OPTIONS` (if any options), `.SH ENVIRONMENT`, `.SH EXIT CODES`, `.SH SEE ALSO`.
3. `cmd_man(command_name, root_cmd, prog_name, version)` looks up `command_name` in `root_cmd`'s subcommands. Falls back to `KNOWN_BUILTINS`. Returns `Err(ShellError::UnknownCommand)` if not found in either.
4. The EXIT CODES section is static and contains all 10 entries from the spec.
5. The ENVIRONMENT section is static and contains the four standard env vars.

### Roff Conventions

- `.TH` title: `"{PROG}-{COMMAND}"` in uppercase, section `"1"`, date in `"%Y-%m-%d"` format, package label `"{prog} {version}"`, manual label `"{prog} Manual"`.
- NAME description: first line of `about` text, trailing period stripped.
- SYNOPSIS option format: `[\-\-flag \fITYPE\fR]` for optional options; `\-\-flag \fITYPE\fR` for required. Flags only: `[\-\-flag]`.
- SYNOPSIS positional format: `\fIMETA\fR` for required; `[\fIMETA\fR]` for optional.
- OPTIONS: `.TP` separator, `\fB\-\-flag\fR \fITYPE\fR` header line, help text on next line, `Default: {value}.` if non-flag default exists.
- Hyphens in roff must be `\-` (not `-`) in option names for man page correctness.
- Backslashes in help text must be doubled: `\\`.

---

## RED — Write Failing Tests First

Add to the `#[cfg(test)]` block in `src/shell.rs`:

```rust
    fn make_exec_cmd() -> clap::Command {
        clap::Command::new("exec")
            .about("Execute an apcore module")
            .arg(
                clap::Arg::new("module_id")
                    .value_name("MODULE_ID")
                    .required(true)
                    .help("Module ID to execute"),
            )
            .arg(
                clap::Arg::new("format")
                    .long("format")
                    .value_name("FORMAT")
                    .help("Output format")
                    .default_value("table"),
            )
    }

    // --- build_synopsis ---

    #[test]
    fn test_build_synopsis_no_cmd() {
        let synopsis = build_synopsis(None, "apcore-cli", "exec");
        assert!(synopsis.contains("apcore-cli"));
        assert!(synopsis.contains("exec"));
    }

    #[test]
    fn test_build_synopsis_required_positional_no_brackets() {
        let cmd = make_exec_cmd();
        let synopsis = build_synopsis(Some(&cmd), "apcore-cli", "exec");
        // MODULE_ID is required — must appear without brackets
        assert!(synopsis.contains("MODULE_ID"), "synopsis: {synopsis}");
        assert!(!synopsis.contains("[\\fIMODULE_ID\\fR]"), "required arg must not have brackets");
    }

    #[test]
    fn test_build_synopsis_optional_option_has_brackets() {
        let cmd = make_exec_cmd();
        let synopsis = build_synopsis(Some(&cmd), "apcore-cli", "exec");
        // --format is optional — must appear with brackets
        assert!(synopsis.contains('['), "optional option must be wrapped in brackets");
    }

    // --- generate_man_page ---

    #[test]
    fn test_generate_man_page_contains_th() {
        let cmd = make_exec_cmd();
        let page = generate_man_page("exec", Some(&cmd), "apcore-cli", "0.2.0");
        assert!(page.contains(".TH"), "man page must have .TH header");
    }

    #[test]
    fn test_generate_man_page_contains_sh_name() {
        let cmd = make_exec_cmd();
        let page = generate_man_page("exec", Some(&cmd), "apcore-cli", "0.2.0");
        assert!(page.contains(".SH NAME"), "man page must have NAME section");
    }

    #[test]
    fn test_generate_man_page_contains_sh_synopsis() {
        let cmd = make_exec_cmd();
        let page = generate_man_page("exec", Some(&cmd), "apcore-cli", "0.2.0");
        assert!(page.contains(".SH SYNOPSIS"), "man page must have SYNOPSIS section");
    }

    #[test]
    fn test_generate_man_page_contains_exit_codes() {
        let cmd = make_exec_cmd();
        let page = generate_man_page("exec", Some(&cmd), "apcore-cli", "0.2.0");
        assert!(page.contains(".SH EXIT CODES"), "man page must have EXIT CODES section");
        // Spot-check a few exit code values
        assert!(page.contains("\\fB0\\fR"), "must contain exit code 0");
        assert!(page.contains("\\fB44\\fR"), "must contain exit code 44");
        assert!(page.contains("\\fB130\\fR"), "must contain exit code 130");
    }

    #[test]
    fn test_generate_man_page_contains_environment() {
        let cmd = make_exec_cmd();
        let page = generate_man_page("exec", Some(&cmd), "apcore-cli", "0.2.0");
        assert!(page.contains(".SH ENVIRONMENT"), "man page must have ENVIRONMENT section");
        assert!(page.contains("APCORE_EXTENSIONS_ROOT"));
        assert!(page.contains("APCORE_CLI_LOGGING_LEVEL"));
    }

    #[test]
    fn test_generate_man_page_contains_see_also() {
        let cmd = make_exec_cmd();
        let page = generate_man_page("exec", Some(&cmd), "apcore-cli", "0.2.0");
        assert!(page.contains(".SH SEE ALSO"), "man page must have SEE ALSO section");
        assert!(page.contains("apcore-cli"));
    }

    #[test]
    fn test_generate_man_page_th_includes_prog_and_version() {
        let cmd = make_exec_cmd();
        let page = generate_man_page("exec", Some(&cmd), "apcore-cli", "0.2.0");
        let th_line = page.lines().find(|l| l.starts_with(".TH")).unwrap();
        assert!(th_line.contains("APCORE-CLI-EXEC"), "TH must contain uppercased title");
        assert!(th_line.contains("0.2.0"), "TH must contain version");
    }

    #[test]
    fn test_generate_man_page_name_uses_description() {
        let cmd = make_exec_cmd();
        let page = generate_man_page("exec", Some(&cmd), "apcore-cli", "0.2.0");
        // NAME section line should contain the about text
        assert!(page.contains("Execute an apcore module"), "NAME must use about text");
    }

    #[test]
    fn test_generate_man_page_no_description_section_when_no_long_help() {
        // make_exec_cmd uses .about() only; no .long_about().
        // The DESCRIPTION section should still appear (about is used as description).
        // This test verifies the section is present but doesn't require long_about.
        let cmd = make_exec_cmd();
        let page = generate_man_page("exec", Some(&cmd), "apcore-cli", "0.2.0");
        assert!(page.contains(".SH DESCRIPTION"));
    }

    // --- cmd_man ---

    #[test]
    fn test_cmd_man_known_builtin_returns_ok() {
        let root = clap::Command::new("apcore-cli")
            .subcommand(make_exec_cmd());
        // "list" is in KNOWN_BUILTINS but not in root subcommands — must still return Ok
        let result = cmd_man("list", &root, "apcore-cli", "0.2.0");
        assert!(result.is_ok(), "known builtin 'list' must return Ok");
    }

    #[test]
    fn test_cmd_man_registered_subcommand_returns_ok() {
        let root = clap::Command::new("apcore-cli")
            .subcommand(make_exec_cmd());
        let result = cmd_man("exec", &root, "apcore-cli", "0.2.0");
        assert!(result.is_ok(), "registered subcommand 'exec' must return Ok");
        let page = result.unwrap();
        assert!(page.contains(".TH"));
    }

    #[test]
    fn test_cmd_man_unknown_command_returns_err() {
        let root = clap::Command::new("apcore-cli");
        let result = cmd_man("nonexistent", &root, "apcore-cli", "0.2.0");
        assert!(result.is_err());
        match result.unwrap_err() {
            ShellError::UnknownCommand(name) => assert_eq!(name, "nonexistent"),
        }
    }

    #[test]
    fn test_cmd_man_exec_contains_options_section() {
        let root = clap::Command::new("apcore-cli")
            .subcommand(make_exec_cmd());
        let page = cmd_man("exec", &root, "apcore-cli", "0.2.0").unwrap();
        assert!(page.contains(".SH OPTIONS"), "exec man page must have OPTIONS section");
    }
```

Run `cargo test test_build_synopsis test_generate_man_page test_cmd_man` — all fail because functions do not exist yet.

---

## GREEN — Implement

Implement all three functions in `src/shell.rs`.

### `build_synopsis`

```rust
/// Build the roff SYNOPSIS line from a clap Command's arguments.
pub fn build_synopsis(
    cmd: Option<&clap::Command>,
    prog_name: &str,
    command_name: &str,
) -> String {
    let Some(cmd) = cmd else {
        return format!("\\fB{prog_name} {command_name}\\fR [OPTIONS]");
    };

    let mut parts = vec![format!("\\fB{prog_name} {command_name}\\fR")];

    for arg in cmd.get_arguments() {
        // Skip help/version flags injected by clap
        let id = arg.get_id().as_str();
        if id == "help" || id == "version" {
            continue;
        }

        let is_positional = arg.get_long().is_none() && arg.get_short().is_none();
        let is_required = arg.is_required_set();

        if is_positional {
            let meta = arg.get_value_names()
                .and_then(|v| v.first().copied())
                .unwrap_or("ARG");
            if is_required {
                parts.push(format!("\\fI{meta}\\fR"));
            } else {
                parts.push(format!("[\\fI{meta}\\fR]"));
            }
        } else {
            let flag = if let Some(long) = arg.get_long() {
                format!("\\-\\-{long}")
            } else {
                format!("\\-{}", arg.get_short().unwrap())
            };
            let is_flag = arg.get_num_args().map_or(false, |r| r.max_values() == 0);
            if is_flag {
                parts.push(format!("[{flag}]"));
            } else {
                let type_name = arg.get_value_names()
                    .and_then(|v| v.first().copied())
                    .unwrap_or("VALUE");
                if is_required {
                    parts.push(format!("{flag} \\fI{type_name}\\fR"));
                } else {
                    parts.push(format!("[{flag} \\fI{type_name}\\fR]"));
                }
            }
        }
    }

    parts.join(" ")
}
```

### `generate_man_page`

```rust
/// Build a complete roff man page string for a CLI subcommand.
pub fn generate_man_page(
    command_name: &str,
    cmd: Option<&clap::Command>,
    prog_name: &str,
    version: &str,
) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Date in YYYY-MM-DD format (no chrono dep — use std)
    let today = {
        // Simple UTC date approximation via UNIX timestamp
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let days = secs / 86400;
        // Use a deterministic placeholder; real impl may use chrono if available
        // For now: format as fixed string derived from env!("CARGO_PKG_VERSION")
        // Acceptable since man page date is informational only.
        format_roff_date(days)
    };

    let title = format!("{}-{}", prog_name, command_name).to_uppercase();
    let pkg_label = format!("{prog_name} {version}");
    let manual_label = format!("{prog_name} Manual");

    let mut sections: Vec<String> = Vec::new();

    // .TH
    sections.push(format!(
        ".TH \"{title}\" \"1\" \"{today}\" \"{pkg_label}\" \"{manual_label}\""
    ));

    // .SH NAME
    sections.push(".SH NAME".to_string());
    let desc = cmd
        .and_then(|c| c.get_about())
        .map(|s| s.to_string())
        .unwrap_or_else(|| command_name.to_string());
    let name_desc = desc.lines().next().unwrap_or("").trim_end_matches('.');
    sections.push(format!("{prog_name}-{command_name} \\- {name_desc}"));

    // .SH SYNOPSIS
    sections.push(".SH SYNOPSIS".to_string());
    sections.push(build_synopsis(cmd, prog_name, command_name));

    // .SH DESCRIPTION (using about text)
    if let Some(about) = cmd.and_then(|c| c.get_about()) {
        sections.push(".SH DESCRIPTION".to_string());
        let escaped = about.to_string()
            .replace('\\', "\\\\")
            .replace('-', "\\-");
        sections.push(escaped);
    }

    // .SH OPTIONS (only if command has named options)
    if let Some(c) = cmd {
        let options: Vec<_> = c.get_arguments()
            .filter(|a| a.get_long().is_some() || a.get_short().is_some())
            .filter(|a| a.get_id().as_str() != "help" && a.get_id().as_str() != "version")
            .collect();

        if !options.is_empty() {
            sections.push(".SH OPTIONS".to_string());
            for arg in options {
                let flag_parts: Vec<String> = {
                    let mut fp = Vec::new();
                    if let Some(short) = arg.get_short() {
                        fp.push(format!("\\-{short}"));
                    }
                    if let Some(long) = arg.get_long() {
                        fp.push(format!("\\-\\-{long}"));
                    }
                    fp
                };
                let flag_str = flag_parts.join(", ");

                let is_flag = arg.get_num_args().map_or(false, |r| r.max_values() == 0);
                sections.push(".TP".to_string());
                if is_flag {
                    sections.push(format!("\\fB{flag_str}\\fR"));
                } else {
                    let type_name = arg.get_value_names()
                        .and_then(|v| v.first().copied())
                        .unwrap_or("VALUE");
                    sections.push(format!("\\fB{flag_str}\\fR \\fI{type_name}\\fR"));
                }
                if let Some(help) = arg.get_help() {
                    sections.push(help.to_string());
                }
                if let Some(default) = arg.get_default_values().first() {
                    if !is_flag {
                        sections.push(format!("Default: {}.", default.to_string_lossy()));
                    }
                }
            }
        }
    }

    // .SH ENVIRONMENT (static)
    sections.push(".SH ENVIRONMENT".to_string());
    let env_entries = [
        ("APCORE_EXTENSIONS_ROOT",
         "Path to the apcore extensions directory. Overrides the default \\fI./extensions\\fR."),
        ("APCORE_CLI_AUTO_APPROVE",
         "Set to \\fB1\\fR to bypass approval prompts for modules that require human-in-the-loop confirmation."),
        ("APCORE_CLI_LOGGING_LEVEL",
         "CLI-specific logging verbosity. One of: DEBUG, INFO, WARNING, ERROR. \
          Takes priority over \\fBAPCORE_LOGGING_LEVEL\\fR. Default: WARNING."),
        ("APCORE_LOGGING_LEVEL",
         "Global apcore logging verbosity. One of: DEBUG, INFO, WARNING, ERROR. \
          Used as fallback when \\fBAPCORE_CLI_LOGGING_LEVEL\\fR is not set. Default: WARNING."),
    ];
    for (name, desc) in env_entries {
        sections.push(".TP".to_string());
        sections.push(format!("\\fB{name}\\fR"));
        sections.push(desc.to_string());
    }

    // .SH EXIT CODES (static — full table from spec)
    sections.push(".SH EXIT CODES".to_string());
    let exit_codes = [
        ("0",   "Success."),
        ("1",   "Module execution error."),
        ("2",   "Invalid CLI input or missing argument."),
        ("44",  "Module not found, disabled, or failed to load."),
        ("45",  "Input failed JSON Schema validation."),
        ("46",  "Approval denied, timed out, or no interactive terminal available."),
        ("47",  "Configuration error (extensions directory not found or unreadable)."),
        ("48",  "Schema contains a circular \\fB$ref\\fR."),
        ("77",  "ACL denied \\- insufficient permissions for this module."),
        ("130", "Execution cancelled by user (SIGINT / Ctrl\\-C)."),
    ];
    for (code, meaning) in exit_codes {
        sections.push(format!(".TP\n\\fB{code}\\fR\n{meaning}"));
    }

    // .SH SEE ALSO
    sections.push(".SH SEE ALSO".to_string());
    let see_also = [
        format!("\\fB{prog_name}\\fR(1)"),
        format!("\\fB{prog_name}\\-list\\fR(1)"),
        format!("\\fB{prog_name}\\-describe\\fR(1)"),
        format!("\\fB{prog_name}\\-completion\\fR(1)"),
    ];
    sections.push(see_also.join(", "));

    sections.join("\n")
}

/// Format Unix epoch days as YYYY-MM-DD without external crates.
fn format_roff_date(days_since_epoch: u64) -> String {
    // Gregorian calendar approximation — sufficient for a man page date.
    let mut remaining = days_since_epoch;
    let mut year = 1970u32;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let days_in_year = if leap { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let month_days = [31u64, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u32;
    for &d in &month_days {
        if remaining < d { break; }
        remaining -= d;
        month += 1;
    }
    let day = remaining + 1;
    format!("{year:04}-{month:02}-{day:02}")
}
```

### `cmd_man`

```rust
/// Handler: look up a subcommand and return its roff man page.
///
/// Returns `Err(ShellError::UnknownCommand)` if `command_name` is not found
/// among `root_cmd`'s subcommands and is not in `KNOWN_BUILTINS`.
pub fn cmd_man(
    command_name: &str,
    root_cmd: &clap::Command,
    prog_name: &str,
    version: &str,
) -> Result<String, ShellError> {
    // Try live subcommand tree first
    let cmd_opt = root_cmd.get_subcommands()
        .find(|c| c.get_name() == command_name);

    // Fall back to known built-ins (commands that may not be wired yet)
    if cmd_opt.is_none() && !KNOWN_BUILTINS.contains(&command_name) {
        return Err(ShellError::UnknownCommand(command_name.to_string()));
    }

    Ok(generate_man_page(command_name, cmd_opt, prog_name, version))
}
```

Run `cargo test test_build_synopsis test_generate_man_page test_cmd_man` — all pass.

---

## REFACTOR

- Extract `exit_codes` and `env_entries` arrays to module-level `const` items so they can be inspected in tests without calling `generate_man_page`.
- Confirm `format_roff_date` produces a valid `YYYY-MM-DD` string for today's date by adding a quick assertion test.
- Run `cargo clippy -- -D warnings` on `src/shell.rs`; fix any warnings (likely: use of `format!` for const strings).
- Check all `unwrap_or` calls — none should panic on valid clap metadata.

---

## Verification

```bash
cargo test test_build_synopsis test_generate_man_page test_cmd_man 2>&1
# Expected: 13 tests pass, 0 fail.

cargo clippy -- -D warnings 2>&1
# Expected: no warnings in src/shell.rs.
```
