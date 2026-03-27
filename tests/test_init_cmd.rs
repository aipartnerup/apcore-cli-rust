// Integration tests for init_cmd (FE-10).

use std::fs;

use tempfile::TempDir;

#[test]
fn test_convention_style_creates_file() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("cmds");
    let dir_str = dir.to_str().unwrap();

    let cmd = apcore_cli::init_cmd::init_command();
    let matches = cmd
        .try_get_matches_from(vec![
            "init",
            "module",
            "greet",
            "--style",
            "convention",
            "--dir",
            dir_str,
        ])
        .unwrap();
    apcore_cli::init_cmd::handle_init(&matches);

    let file = dir.join("greet.rs");
    assert!(file.exists(), "convention file must be created");
    let content = fs::read_to_string(&file).unwrap();
    assert!(
        content.contains("pub fn greet("),
        "must contain function definition"
    );
}

#[test]
fn test_decorator_style_creates_file_with_module() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("ext");
    let dir_str = dir.to_str().unwrap();

    let cmd = apcore_cli::init_cmd::init_command();
    let matches = cmd
        .try_get_matches_from(vec![
            "init",
            "module",
            "math.add",
            "--style",
            "decorator",
            "--dir",
            dir_str,
        ])
        .unwrap();
    apcore_cli::init_cmd::handle_init(&matches);

    let file = dir.join("math_add.rs");
    assert!(file.exists(), "decorator file must be created");
    let content = fs::read_to_string(&file).unwrap();
    assert!(
        content.contains("impl Module for"),
        "must contain impl Module"
    );
}

#[test]
fn test_binding_style_creates_yaml() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("bindings");
    let dir_str = dir.to_str().unwrap();

    let cmd = apcore_cli::init_cmd::init_command();
    let matches = cmd
        .try_get_matches_from(vec![
            "init",
            "module",
            "text.upper",
            "--style",
            "binding",
            "--dir",
            dir_str,
        ])
        .unwrap();
    apcore_cli::init_cmd::handle_init(&matches);

    let yaml_file = dir.join("text_upper.binding.yaml");
    assert!(yaml_file.exists(), "YAML binding must be created");
    let yaml_content = fs::read_to_string(&yaml_file).unwrap();
    assert!(
        yaml_content.contains("text.upper"),
        "YAML must contain module id"
    );

    // Companion Rust file is created at commands/text.rs
    // (relative to CWD).
    let rs_file = std::path::Path::new("commands").join("text.rs");
    assert!(rs_file.exists(), "companion Rust file must be created");
    // Clean up the companion file.
    let _ = fs::remove_file(&rs_file);
    let _ = fs::remove_dir("commands");
}

#[test]
fn test_convention_dotted_id_has_cli_group() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("cmds2");
    let dir_str = dir.to_str().unwrap();

    let cmd = apcore_cli::init_cmd::init_command();
    let matches = cmd
        .try_get_matches_from(vec![
            "init",
            "module",
            "math.add",
            "--style",
            "convention",
            "--dir",
            dir_str,
        ])
        .unwrap();
    apcore_cli::init_cmd::handle_init(&matches);

    let file = dir.join("math").join("add.rs");
    let content = fs::read_to_string(&file).unwrap();
    assert!(
        content.contains("pub const CLI_GROUP: &str = \"math\""),
        "dotted module_id must produce CLI_GROUP"
    );
}

#[test]
fn test_description_flag() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("ext2");
    let dir_str = dir.to_str().unwrap();

    let cmd = apcore_cli::init_cmd::init_command();
    let matches = cmd
        .try_get_matches_from(vec![
            "init",
            "module",
            "demo.hello",
            "--style",
            "decorator",
            "--dir",
            dir_str,
            "-d",
            "My custom description",
        ])
        .unwrap();
    apcore_cli::init_cmd::handle_init(&matches);

    let file = dir.join("demo_hello.rs");
    let content = fs::read_to_string(&file).unwrap();
    assert!(
        content.contains("My custom description"),
        "custom description must appear in file"
    );
}
