// apcore-cli example — text.upper
// Demonstrates a string uppercase module.
//
// Run via the apcore-cli binary:
//   APCORE_EXTENSIONS_ROOT=examples/extensions apcore-cli text.upper --text "hello apcore"

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

// ---------------------------------------------------------------------------
// Input / Output schemas
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Input {
    /// Input string to convert to uppercase.
    pub text: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Output {
    /// The uppercased result.
    pub result: String,
}

// ---------------------------------------------------------------------------
// Module implementation
// ---------------------------------------------------------------------------

/// Convert a string to uppercase.
pub struct TextUpper;

impl TextUpper {
    pub const MODULE_ID: &'static str = "text.upper";
    pub const DESCRIPTION: &'static str = "Convert a string to uppercase";

    pub fn execute(input: Input) -> Output {
        Output { result: input.text.to_uppercase() }
    }
}

fn main() {
    let input = Input { text: "hello apcore".to_string() };
    let output = TextUpper::execute(input);
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
