// apcore-cli example — math.add
// Demonstrates a simple integer addition module.
//
// Run via the apcore-cli binary:
//   APCORE_EXTENSIONS_ROOT=examples/extensions apcore-cli math.add --a 3 --b 4

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

// ---------------------------------------------------------------------------
// Input / Output schemas
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Input {
    /// First operand.
    pub a: i64,
    /// Second operand.
    pub b: i64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Output {
    /// The sum of a and b.
    pub sum: i64,
}

// ---------------------------------------------------------------------------
// Module implementation
// ---------------------------------------------------------------------------

/// Add two integers and return their sum.
pub struct MathAdd;

impl MathAdd {
    pub const MODULE_ID: &'static str = "math.add";
    pub const DESCRIPTION: &'static str = "Add two integers and return their sum";

    /// Execute the module: compute a + b.
    pub fn execute(input: Input) -> Output {
        Output { sum: input.a + input.b }
    }
}

fn main() {
    // Standalone entry point for quick manual testing.
    let input = Input { a: 3, b: 4 };
    let output = MathAdd::execute(input);
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
