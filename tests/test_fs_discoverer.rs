//! Behavioral tests for the `fs_discoverer` module.

use std::fs;
use std::sync::Arc;

use apcore::registry::registry::{DiscoveredModule, Discoverer};
use apcore_cli::FsDiscoverer;
use tempfile::TempDir;

#[test]
fn fs_discoverer_construct_from_path() {
    let tmp = TempDir::new().unwrap();
    let _ = FsDiscoverer::new(tmp.path());
}

#[test]
fn fs_discoverer_executables_snapshot_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let discoverer = FsDiscoverer::new(tmp.path());
    let snap = discoverer.executables_snapshot();
    assert!(snap.is_empty());
}

fn write_module_json(root: &std::path::Path, name: &str, body: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("module.json"), body).unwrap();
}

#[tokio::test]
async fn fs_discoverer_discover_skips_malformed_module_json() {
    // Regression for review #14: a single malformed module.json must not
    // abort the whole pass. Sibling well-formed modules must still load.
    let tmp = TempDir::new().unwrap();

    write_module_json(
        tmp.path(),
        "good",
        r#"{"name":"good.mod","description":"ok","input_schema":{},"output_schema":{}}"#,
    );
    write_module_json(tmp.path(), "broken", "{ not valid json");
    write_module_json(
        tmp.path(),
        "alsogood",
        r#"{"name":"also.good","description":"ok","input_schema":{},"output_schema":{}}"#,
    );

    let discoverer = Arc::new(FsDiscoverer::new(tmp.path()));
    let modules: Vec<DiscoveredModule> = discoverer
        .discover(&[])
        .await
        .expect("discover must not propagate the parse failure");

    let names: Vec<&str> = modules.iter().map(|m| m.name.as_str()).collect();
    assert!(
        names.contains(&"good.mod"),
        "well-formed module must load alongside a malformed sibling, got {names:?}"
    );
    assert!(
        names.contains(&"also.good"),
        "modules after the malformed entry must still load, got {names:?}"
    );
}

#[tokio::test]
async fn fs_discoverer_discover_skips_unreadable_module_json() {
    // Regression for review #14: an IO error on one module.json (here, an
    // empty file path that fails JSON parse) must not abort the loop.
    let tmp = TempDir::new().unwrap();

    write_module_json(tmp.path(), "empty", "");
    write_module_json(
        tmp.path(),
        "ok",
        r#"{"name":"ok.mod","description":"ok","input_schema":{},"output_schema":{}}"#,
    );

    let discoverer = Arc::new(FsDiscoverer::new(tmp.path()));
    let modules: Vec<DiscoveredModule> = discoverer
        .discover(&[])
        .await
        .expect("must tolerate empty file");
    let names: Vec<&str> = modules.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"ok.mod"), "got {names:?}");
}
