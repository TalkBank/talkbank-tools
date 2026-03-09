#!/bin/bash
# Regenerate tree-sitter grammar and update bindings
#
# Run this after any grammar.js change to regenerate parser.c,
# node type constants, and run tests.
#
# Usage: ./scripts/update-tree-sitter.sh

set -e

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GRAMMAR_DIR="$REPO_ROOT/grammar"

echo "==> Regenerating tree-sitter grammar..."
cd "$GRAMMAR_DIR"
tree-sitter generate

echo "==> Running tree-sitter tests..."
tree-sitter test

echo "==> Generating node type constants..."
node "$REPO_ROOT/scripts/generate-node-types.js" "$GRAMMAR_DIR" > "$REPO_ROOT/crates/talkbank-parser/src/node_types.rs"

echo "==> Running Rust tests..."
cd "$REPO_ROOT"
cargo nextest run -p talkbank-parser
cargo nextest run -p talkbank-parser-tests

echo "Tree-sitter update complete!"
