#!/bin/bash
# 01-baseline.sh — Capture baseline metrics for the current grammar
set -euo pipefail

GRAMMAR_DIR="$(cd "$(dirname "$0")/../../grammar" && pwd)"
RESULTS_DIR="$(dirname "$0")/results"
mkdir -p "$RESULTS_DIR"

echo "=== Baseline Metrics ==="
echo "Grammar: $GRAMMAR_DIR/grammar.js"

# Grammar size
echo ""
echo "--- Grammar size ---"
wc -l "$GRAMMAR_DIR/grammar.js" | tee "$RESULTS_DIR/baseline-grammar-lines.txt"

# Generated parser size
echo ""
echo "--- Parser.c size ---"
wc -l "$GRAMMAR_DIR/src/parser.c" | tee "$RESULTS_DIR/baseline-parser-c-lines.txt"

# grammar.json rule count
echo ""
echo "--- Rule count ---"
python3 -c "
import json
with open('$GRAMMAR_DIR/src/grammar.json') as f:
    g = json.load(f)
print(f'Rules: {len(g[\"rules\"])}')
print(f'Conflicts: {len(g.get(\"conflicts\", []))}')
print(f'Extras: {len(g.get(\"extras\", []))}')
print(f'Supertypes: {len(g.get(\"supertypes\", []))}')
" | tee "$RESULTS_DIR/baseline-rule-stats.txt"

# Generate and capture conflict count from tree-sitter generate
echo ""
echo "--- Conflict count (from generate) ---"
cd "$GRAMMAR_DIR"
tree-sitter generate 2>&1 | tee "$RESULTS_DIR/baseline-generate-output.txt"

# Parse reference corpus
echo ""
echo "--- Reference corpus parse ---"
CORPUS_DIR="$(cd "$GRAMMAR_DIR/../corpus/reference" && pwd)"
file_count=0
error_count=0
for f in "$CORPUS_DIR"/**/*.cha; do
    result=$(tree-sitter parse "$f" 2>&1)
    file_count=$((file_count + 1))
    if echo "$result" | grep -q '(ERROR'; then
        error_count=$((error_count + 1))
    fi
done
echo "Files: $file_count, Files with ERROR nodes: $error_count" | tee "$RESULTS_DIR/baseline-corpus-parse.txt"

echo ""
echo "Baseline captured in $RESULTS_DIR/"
