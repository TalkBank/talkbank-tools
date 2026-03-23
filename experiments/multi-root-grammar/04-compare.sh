#!/bin/bash
# 04-compare.sh — Compare baseline vs variant metrics
set -euo pipefail

RESULTS_DIR="$(dirname "$0")/results"

echo "=== Baseline vs Multi-Root Variant ==="
echo ""

echo "--- Grammar lines ---"
echo -n "  Baseline: "; cat "$RESULTS_DIR/baseline-grammar-lines.txt" 2>/dev/null || echo "not measured"
echo ""

echo "--- Parser.c lines ---"
baseline_pc=$(cat "$RESULTS_DIR/baseline-parser-c-lines.txt" 2>/dev/null | awk '{print $1}')
variant_pc=$(cat "$RESULTS_DIR/variant-parser-c-lines.txt" 2>/dev/null | awk '{print $1}')
echo "  Baseline: ${baseline_pc:-?}"
echo "  Variant:  ${variant_pc:-?}"
if [ -n "$baseline_pc" ] && [ -n "$variant_pc" ]; then
    diff=$((variant_pc - baseline_pc))
    pct=$(python3 -c "print(f'{100*$diff/$baseline_pc:.1f}')")
    echo "  Delta:    ${diff} lines (${pct}%)"
fi

echo ""
echo "--- Rule stats ---"
echo "  Baseline:"
cat "$RESULTS_DIR/baseline-rule-stats.txt" 2>/dev/null | sed 's/^/    /'
echo "  Variant:"
cat "$RESULTS_DIR/variant-rule-stats.txt" 2>/dev/null | sed 's/^/    /'

echo ""
echo "--- Generate output (warnings/conflicts) ---"
echo "  Baseline:"
cat "$RESULTS_DIR/baseline-generate-output.txt" 2>/dev/null | sed 's/^/    /' || echo "    (no output)"
echo "  Variant:"
cat "$RESULTS_DIR/variant-generate-output.txt" 2>/dev/null | sed 's/^/    /' || echo "    (no output)"

echo ""
echo "--- Corpus parse (baseline) ---"
cat "$RESULTS_DIR/baseline-corpus-parse.txt" 2>/dev/null | sed 's/^/  /' || echo "  not measured"
