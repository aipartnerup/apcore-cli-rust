// apcore-cli example — sysutil.env
// Demonstrates reading an environment variable.
//
// Run via the apcore-cli binary:
//   APCORE_EXTENSIONS_ROOT=examples/extensions apcore-cli sysutil.env --name HOME

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

// ---------------------------------------------------------------------------
// Input / Output schemas
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Input {
    /// Environment variable name to read.
    pub name: String,
    /// Value to return if the variable is not set (default: "").
    #[serde(default)]
    pub default: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Output {
    /// The environment variable name.
    pub name: String,
    /// The resolved value (env var value or `default`).
    pub value: String,
    /// Whether the variable was actually set in the environment.
    pub exists: bool,
}

// ---------------------------------------------------------------------------
// Module implementation
// ---------------------------------------------------------------------------

/// Read an environment variable value.
pub struct SysutilEnv;

impl SysutilEnv {
    pub const MODULE_ID: &'static str = "sysutil.env";
    pub const DESCRIPTION: &'static str = "Read an environment variable value";

    pub fn execute(input: Input) -> Output {
        let exists = std::env::var(&input.name).is_ok();
        let value = std::env::var(&input.name).unwrap_or(input.default);
        Output {
            name: input.name,
            value,
            exists,
        }
    }
}

fn main() {
    let input = Input {
        name: "HOME".to_string(),
        default: String::new(),
    };
    let output = SysutilEnv::execute(input);
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
