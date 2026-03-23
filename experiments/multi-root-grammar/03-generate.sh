#!/bin/bash
# 03-generate.sh — Run tree-sitter generate on the multi-root variant
set -euo pipefail

EXPERIMENT_DIR="$(dirname "$0")"
VARIANT_DIR="$EXPERIMENT_DIR/variant"
RESULTS_DIR="$EXPERIMENT_DIR/results"
mkdir -p "$RESULTS_DIR"

echo "=== Generating multi-root variant ==="
cd "$VARIANT_DIR"

# Generate and capture output (including conflict warnings)
tree-sitter generate 2>&1 | tee "$RESULTS_DIR/variant-generate-output.txt"
gen_exit=$?

if [ $gen_exit -ne 0 ]; then
    echo ""
    echo "FAILED: tree-sitter generate exited with $gen_exit"
    echo "Check $RESULTS_DIR/variant-generate-output.txt for details"
    exit 1
fi

echo ""
echo "--- Variant parser.c size ---"
wc -l "$VARIANT_DIR/src/parser.c" | tee "$RESULTS_DIR/variant-parser-c-lines.txt"

echo ""
echo "--- Variant rule count ---"
python3 -c "
import json
with open('$VARIANT_DIR/src/grammar.json') as f:
    g = json.load(f)
print(f'Rules: {len(g[\"rules\"])}')
print(f'Conflicts: {len(g.get(\"conflicts\", []))}')
" | tee "$RESULTS_DIR/variant-rule-stats.txt"

echo ""
echo "Generation succeeded. Next: run 04-compare.sh"
