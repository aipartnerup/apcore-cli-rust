// apcore-cli -- Scaffolding commands (init module).
// Protocol spec: FE-10

use std::fs;
use std::path::Path;

/// Register the `init` subcommand with its `module` sub-subcommand.
pub fn init_command() -> clap::Command {
    clap::Command::new("init")
        .about("Scaffolding commands")
        .subcommand(
            clap::Command::new("module")
                .about("Create a new module from a template")
                .arg(clap::Arg::new("module_id").required(true))
                .arg(
                    clap::Arg::new("style")
                        .long("style")
                        .default_value("convention")
                        .value_parser(["decorator", "convention", "binding"]),
                )
                .arg(clap::Arg::new("dir").long("dir").value_name("PATH"))
                .arg(
                    clap::Arg::new("description")
                        .long("description")
                        .short('d')
                        .default_value("TODO: add description"),
                ),
        )
}

/// Handle the `init` subcommand dispatch.
pub fn handle_init(matches: &clap::ArgMatches) {
    if let Some(("module", sub_m)) = matches.subcommand() {
        let module_id = sub_m.get_one::<String>("module_id").unwrap();
        let style = sub_m.get_one::<String>("style").unwrap();
        let description = sub_m.get_one::<String>("description").unwrap();

        // Parse module_id: split on last dot for prefix/func_name.
        let (prefix, func_name) = match module_id.rfind('.') {
            Some(pos) => (&module_id[..pos], &module_id[pos + 1..]),
            None => (module_id.as_str(), module_id.as_str()),
        };

        match style.as_str() {
            "decorator" => {
                let dir = sub_m
                    .get_one::<String>("dir")
                    .map(|s| s.as_str())
                    .unwrap_or("extensions");
                validate_dir(dir);
                create_decorator_module(module_id, func_name, description, dir);
            }
            "convention" => {
                let dir = sub_m
                    .get_one::<String>("dir")
                    .map(|s| s.as_str())
                    .unwrap_or("commands");
                validate_dir(dir);
                create_convention_module(module_id, prefix, func_name, description, dir);
            }
            "binding" => {
                let dir = sub_m
                    .get_one::<String>("dir")
                    .map(|s| s.as_str())
                    .unwrap_or("bindings");
                validate_dir(dir);
                create_binding_module(module_id, prefix, func_name, description, dir);
            }
            _ => unreachable!(),
        }
    }
}

/// Validate that the output directory does not contain `..` path
/// components, preventing path traversal outside the project directory.
fn validate_dir(dir: &str) {
    if dir.contains("..") {
        eprintln!("Error: Output directory must not contain '..' path components.");
        std::process::exit(2);
    }
}

/// Create a decorator-style module (Python file with @module).
fn create_decorator_module(module_id: &str, func_name: &str, description: &str, dir: &str) {
    let dir_path = Path::new(dir);
    fs::create_dir_all(dir_path).unwrap_or_else(|e| {
        eprintln!(
            "Error: cannot create directory '{}': {e}",
            dir_path.display()
        );
        std::process::exit(2);
    });

    let safe_name = module_id.replace('.', "_");
    let filename = format!("{safe_name}.py");
    let filepath = dir_path.join(&filename);

    let content = format!(
        r#""""Module: {module_id}"""

from apcore import module


@module(id="{module_id}", description="{description}")
def {func_name}() -> dict:
    """{description}"""
    # TODO: implement
    return {{"status": "ok"}}
"#,
    );

    fs::write(&filepath, content).unwrap_or_else(|e| {
        eprintln!("Error: cannot write '{}': {e}", filepath.display());
        std::process::exit(2);
    });

    println!("Created {}", filepath.display());
}

/// Create a convention-style module (plain Python function with
/// CLI_GROUP).
fn create_convention_module(
    module_id: &str,
    prefix: &str,
    func_name: &str,
    description: &str,
    dir: &str,
) {
    // Build the file path: prefix parts become subdirectories.
    // e.g. module_id "ops.deploy" with dir "commands" -> "commands/ops/deploy.py"
    // e.g. module_id "standalone" with dir "commands" -> "commands/standalone.py"
    let filepath = if module_id.contains('.') {
        let parts: Vec<&str> = module_id.split('.').collect();
        let mut p = Path::new(dir).to_path_buf();
        for part in &parts[..parts.len() - 1] {
            p = p.join(part);
        }
        p.join(format!("{}.py", parts[parts.len() - 1]))
    } else {
        Path::new(dir).join(format!("{func_name}.py"))
    };

    if let Some(parent) = filepath.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!("Error: cannot create directory '{}': {e}", parent.display());
            std::process::exit(2);
        });
    }

    // Only emit CLI_GROUP when module_id contains a dot.
    // Use the first prefix segment (before any dots in the prefix).
    let first_segment = prefix.split('.').next().unwrap_or(prefix);
    let group_line = if module_id.contains('.') {
        format!("CLI_GROUP = \"{first_segment}\"\n\n")
    } else {
        String::new()
    };

    let content = format!(
        r#""""{description}"""

{group_line}def {func_name}() -> dict:
    """{description}"""
    # TODO: implement
    return {{"status": "ok"}}
"#,
    );

    fs::write(&filepath, content).unwrap_or_else(|e| {
        eprintln!("Error: cannot write '{}': {e}", filepath.display());
        std::process::exit(2);
    });

    println!("Created {}", filepath.display());
}

/// Create a binding-style module (YAML binding + companion Python
/// file).
fn create_binding_module(
    module_id: &str,
    prefix: &str,
    func_name: &str,
    description: &str,
    dir: &str,
) {
    let dir_path = Path::new(dir);
    fs::create_dir_all(dir_path).unwrap_or_else(|e| {
        eprintln!(
            "Error: cannot create directory '{}': {e}",
            dir_path.display()
        );
        std::process::exit(2);
    });

    // Write YAML binding file: {module_id_with_dots_as_underscores}.binding.yaml
    let safe_name = module_id.replace('.', "_");
    let yaml_filename = format!("{safe_name}.binding.yaml");
    let yaml_filepath = dir_path.join(&yaml_filename);

    // Build the target string: "commands.{prefix}:{func_name}" (keep dots in prefix)
    let target = format!("commands.{prefix}:{func_name}");
    let prefix_underscored = prefix.replace('.', "_");

    let yaml_content = format!(
        r#"bindings:
  - module_id: "{module_id}"
    target: "{target}"
    description: "{description}"
    auto_schema: true
"#,
    );

    fs::write(&yaml_filepath, yaml_content).unwrap_or_else(|e| {
        eprintln!("Error: cannot write '{}': {e}", yaml_filepath.display());
        std::process::exit(2);
    });

    println!("Created {}", yaml_filepath.display());

    // Write companion Python file to commands/{prefix_with_dots_as_underscores}.py
    let py_filename = format!("{prefix_underscored}.py");
    let py_filepath = Path::new("commands").join(&py_filename);

    // Only create if it does not already exist.
    if !py_filepath.exists() {
        if let Some(parent) = py_filepath.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!("Error: cannot create directory '{}': {e}", parent.display());
                std::process::exit(2);
            });
        }

        let py_content = format!(
            r#"def {func_name}() -> dict:
    """{description}"""
    # TODO: implement
    return {{"status": "ok"}}
"#,
        );

        fs::write(&py_filepath, py_content).unwrap_or_else(|e| {
            eprintln!("Error: cannot write '{}': {e}", py_filepath.display());
            std::process::exit(2);
        });

        println!("Created {}", py_filepath.display());
    }
}

// -------------------------------------------------------------------
// Unit tests
// -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_command_has_module_subcommand() {
        let cmd = init_command();
        let sub = cmd.get_subcommands().find(|c| c.get_name() == "module");
        assert!(sub.is_some(), "init must have 'module' subcommand");
    }

    #[test]
    fn test_init_command_module_has_required_module_id() {
        let cmd = init_command();
        let module_cmd = cmd
            .get_subcommands()
            .find(|c| c.get_name() == "module")
            .expect("module subcommand");
        let arg = module_cmd
            .get_arguments()
            .find(|a| a.get_id() == "module_id");
        assert!(arg.is_some(), "must have module_id arg");
        assert!(arg.unwrap().is_required_set(), "module_id must be required");
    }

    #[test]
    fn test_init_command_module_has_style_flag() {
        let cmd = init_command();
        let module_cmd = cmd
            .get_subcommands()
            .find(|c| c.get_name() == "module")
            .expect("module subcommand");
        let style = module_cmd.get_arguments().find(|a| a.get_id() == "style");
        assert!(style.is_some(), "must have --style flag");
    }

    #[test]
    fn test_init_command_module_has_dir_flag() {
        let cmd = init_command();
        let module_cmd = cmd
            .get_subcommands()
            .find(|c| c.get_name() == "module")
            .expect("module subcommand");
        let dir = module_cmd.get_arguments().find(|a| a.get_id() == "dir");
        assert!(dir.is_some(), "must have --dir flag");
    }

    #[test]
    fn test_init_command_module_has_description_flag() {
        let cmd = init_command();
        let module_cmd = cmd
            .get_subcommands()
            .find(|c| c.get_name() == "module")
            .expect("module subcommand");
        let desc = module_cmd
            .get_arguments()
            .find(|a| a.get_id() == "description");
        assert!(desc.is_some(), "must have --description flag");
    }

    #[test]
    fn test_init_command_parses_valid_args() {
        let cmd = init_command();
        let result =
            cmd.try_get_matches_from(vec!["init", "module", "my.module", "--style", "decorator"]);
        assert!(result.is_ok(), "valid args must parse: {:?}", result.err());
    }
}
