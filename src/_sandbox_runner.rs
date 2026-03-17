// apcore-cli — Internal sandbox runner entry point.
// This module is NOT part of the public API. It is invoked as a subprocess
// by the Sandbox security layer to execute modules in isolation.
// Protocol spec: SEC-04 (Sandbox)

use serde_json::Value;

/// Entry point for the sandboxed subprocess.
///
/// Reads a JSON execution request from stdin, calls the specified executor,
/// and writes the result (or error) as JSON to stdout.
///
/// Environment variables consumed:
/// * `APCORE_SANDBOX_MODULE_ID`  — module to execute
/// * `APCORE_SANDBOX_INPUT_DATA` — JSON-encoded input (base64 for large payloads)
///
/// Exit codes mirror the main CLI conventions (0, 1, 44, 45, …).
pub(crate) async fn run_sandbox_subprocess() -> Result<(), anyhow::Error> {
    // TODO: read module_id and input_data from env / stdin,
    //       instantiate executor, call execute, write JSON result to stdout.
    todo!("run_sandbox_subprocess")
}

/// Serialise the sandbox result for IPC.
pub(crate) fn encode_result(result: &Value) -> String {
    // TODO: serde_json::to_string(result)
    let _ = result;
    todo!("encode_result")
}

/// Deserialise the sandbox result received by the parent process.
pub(crate) fn decode_result(raw: &str) -> Result<Value, serde_json::Error> {
    // TODO: serde_json::from_str(raw)
    let _ = raw;
    todo!("decode_result")
}
