#!/bin/bash
# run.sh — Run the full multi-root grammar experiment
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"

echo "================================================="
echo "Multi-Root Grammar Experiment"
echo "================================================="
echo ""

echo "Step 1: Baseline"
echo "-----------------"
bash "$DIR/01-baseline.sh"
echo ""

echo "Step 2: Create variant"
echo "----------------------"
bash "$DIR/02-modify-grammar.sh"
echo ""

echo "Step 3: Generate variant parser"
echo "-------------------------------"
bash "$DIR/03-generate.sh"
echo ""

echo "Step 4: Compare metrics"
echo "-----------------------"
bash "$DIR/04-compare.sh"
echo ""

echo "Step 5: Test fragment parsing"
echo "-----------------------------"
bash "$DIR/05-test-fragments.sh"
echo ""

echo "================================================="
echo "Experiment complete. Results in experiments/multi-root-grammar/results/"
echo "================================================="
