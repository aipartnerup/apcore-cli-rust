// apcore-cli example — math.multiply
// Demonstrates a simple integer multiplication module.
//
// Run via the apcore-cli binary:
//   APCORE_EXTENSIONS_ROOT=examples/extensions apcore-cli math.multiply --a 6 --b 7

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
    /// The product of a and b.
    pub product: i64,
}

// ---------------------------------------------------------------------------
// Module implementation
// ---------------------------------------------------------------------------

/// Multiply two integers and return their product.
pub struct MathMultiply;

impl MathMultiply {
    pub const MODULE_ID: &'static str = "math.multiply";
    pub const DESCRIPTION: &'static str = "Multiply two integers and return their product";

    /// Execute the module: compute a * b.
    pub fn execute(input: Input) -> Output {
        Output { product: input.a * input.b }
    }
}

fn main() {
    let input = Input { a: 6, b: 7 };
    let output = MathMultiply::execute(input);
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
