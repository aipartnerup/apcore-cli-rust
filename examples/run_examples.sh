#!/usr/bin/env bash
# ============================================================================
# apcore-cli-rust examples — run these from the project root directory
#
# Usage:
#   cd /path/to/apcore-cli-rust
#   bash examples/run_examples.sh
#
# Prerequisites:
#   cargo build --release
# ============================================================================

set -e

BINARY="${BINARY:-./target/release/apcore-cli}"
export APCORE_EXTENSIONS_ROOT=examples/extensions

echo "============================================"
echo " apcore-cli-rust Examples (macOS/Linux)"
echo "============================================"
echo ""

# --- Discovery ---
echo "1. List all modules:"
echo "   \$ apcore-cli list --format json"
"$BINARY" list --format json
echo ""

echo "2. Filter by tag:"
echo "   \$ apcore-cli list --tag math --format json"
"$BINARY" list --tag math --format json
echo ""

echo "3. Describe a module:"
echo "   \$ apcore-cli describe math.add --format json"
"$BINARY" describe math.add --format json
echo ""

# --- Execution ---
echo "4. Execute math.add with CLI flags:"
echo "   \$ apcore-cli math.add --a 42 --b 58"
"$BINARY" math.add --a 42 --b 58
echo ""

echo "5. Execute math.multiply:"
echo "   \$ apcore-cli math.multiply --a 6 --b 7"
"$BINARY" math.multiply --a 6 --b 7
echo ""

echo "6. Execute text.upper:"
echo "   \$ apcore-cli text.upper --text 'hello apcore'"
"$BINARY" text.upper --text 'hello apcore'
echo ""

echo "7. Execute text.reverse:"
echo "   \$ apcore-cli text.reverse --text 'apcore-cli'"
"$BINARY" text.reverse --text 'apcore-cli'
echo ""

echo "8. Execute text.wordcount:"
echo "   \$ apcore-cli text.wordcount --text 'hello world from apcore'"
"$BINARY" text.wordcount --text 'hello world from apcore'
echo ""

# --- STDIN Piping ---
echo "9. Pipe JSON via STDIN:"
echo "   \$ echo '{\"a\": 100, \"b\": 200}' | apcore-cli math.add --input -"
echo '{"a": 100, "b": 200}' | "$BINARY" math.add --input -
echo ""

echo "10. CLI flag overrides STDIN:"
echo "   \$ echo '{\"a\": 1, \"b\": 2}' | apcore-cli math.add --input - --a 999"
echo '{"a": 1, "b": 2}' | "$BINARY" math.add --input - --a 999
echo ""

# --- System modules ---
echo "11. Get system info:"
echo "   \$ apcore-cli sysutil.info"
"$BINARY" sysutil.info
echo ""

echo "12. Read environment variable:"
echo "   \$ apcore-cli sysutil.env --name HOME"
"$BINARY" sysutil.env --name HOME
echo ""

echo "13. Check disk usage:"
echo "   \$ apcore-cli sysutil.disk --path /"
"$BINARY" sysutil.disk --path /
echo ""

# --- Chaining (Unix pipes) ---
echo "14. Chain modules — add result piped through jq:"
echo "   \$ apcore-cli math.add --a 5 --b 10 | jq '.sum'"
RESULT=$("$BINARY" math.add --a 5 --b 10)
echo "   math.add returned: $RESULT"
echo ""

# --- Shell completion ---
echo "15. Generate bash completion:"
echo "   \$ apcore-cli completion bash | head -5"
"$BINARY" completion bash | head -5
echo "   ..."
echo ""

# --- Help ---
echo "16. Module help (auto-generated from schema):"
echo "   \$ apcore-cli math.add --help"
"$BINARY" math.add --help
echo ""

echo "============================================"
echo " All examples completed successfully!"
echo "============================================"
