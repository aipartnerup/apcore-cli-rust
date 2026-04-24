// apcore-cli — Internal sandbox runner entry point.
// This module is NOT part of the public API. It is invoked as a subprocess
// by the Sandbox security layer to execute modules in isolation.
// Protocol spec: SEC-04 (Sandbox)

use serde_json::Value;

/// Default extensions root when `APCORE_EXTENSIONS_ROOT` is absent.
const DEFAULT_EXTENSIONS_ROOT: &str = "./extensions";

/// Entry point for the sandboxed subprocess.
///
/// Reads `module_id` from `argv[2]` (position after `apcore-cli --internal-sandbox-runner`)
/// and `input_data` as JSON from stdin, rebuilds the apcore registry by
/// running the same filesystem discovery as the parent process (driven by
/// `APCORE_EXTENSIONS_ROOT`, which is always propagated by the parent via
/// `SANDBOX_ALLOWED_ENV_PREFIXES`), calls the executor, and writes the JSON
/// result to stdout.
///
/// Exit codes mirror the main CLI conventions (0, 1, 44, 45, …).
pub async fn run_sandbox_subprocess() -> Result<(), anyhow::Error> {
    use tokio::io::AsyncReadExt;

    let module_id = std::env::args()
        .nth(2)
        .ok_or_else(|| anyhow::anyhow!("sandbox runner: missing module_id argument"))?;

    // Read JSON input from stdin.
    let mut stdin_buf = String::new();
    tokio::io::stdin().read_to_string(&mut stdin_buf).await?;
    let input_data: Value = serde_json::from_str(&stdin_buf)?;

    // Rebuild registry by discovering the same extensions tree the parent
    // resolved at startup. Without this, executor.call below would always
    // return MODULE_NOT_FOUND because the registry is empty.
    let extensions_root = std::env::var("APCORE_EXTENSIONS_ROOT")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_EXTENSIONS_ROOT.to_string());
    let registry = apcore::Registry::new();
    let discoverer = crate::fs_discoverer::FsDiscoverer::new(&extensions_root);
    // Propagate discovery failures rather than continuing with an empty
    // registry — otherwise the parent sees a generic MODULE_NOT_FOUND
    // (exit 44) and the real cause (permission denied on extensions root,
    // YAML scan error, etc.) gets buried earlier in the stderr stream.
    registry.discover(&discoverer).await.map_err(|e| {
        anyhow::anyhow!(
            "sandbox runner: discovery failed for extensions root '{}': {}",
            extensions_root,
            e
        )
    })?;
    // Publish discovered executables so binding-style modules can resolve
    // their subprocess entry points when invoked via executor.call.
    crate::cli::set_executables(discoverer.executables_snapshot());

    // Build executor from the now-populated registry.
    let config = apcore::Config::default();
    let executor = apcore::Executor::new(std::sync::Arc::new(registry), config);
    let result = executor.call(&module_id, input_data, None, None).await?;

    // Write JSON result to stdout.
    let encoded = encode_result(&result);
    print!("{encoded}");
    Ok(())
}

/// Serialise the sandbox result for IPC.
pub fn encode_result(result: &Value) -> String {
    serde_json::to_string(result).unwrap_or_else(|_| "null".to_string())
}

/// Deserialise the sandbox result received by the parent process.
pub fn decode_result(raw: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(raw)
}
