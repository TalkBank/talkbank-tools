#!/bin/bash
# 05-test-fragments.sh — Test fragment parsing with the multi-root variant
set -euo pipefail

EXPERIMENT_DIR="$(dirname "$0")"
VARIANT_DIR="$EXPERIMENT_DIR/variant"
RESULTS_DIR="$EXPERIMENT_DIR/results"
FRAGMENTS_DIR="$EXPERIMENT_DIR/fragments"
mkdir -p "$RESULTS_DIR" "$FRAGMENTS_DIR"

# Create test fragments
cat > "$FRAGMENTS_DIR/main-tier.cha" << 'FRAG'
*CHI:	hello world .
FRAG

cat > "$FRAGMENTS_DIR/main-tier-complex.cha" << 'FRAG'
*MOT:	want more cookie [= chocolate chip] ?
FRAG

cat > "$FRAGMENTS_DIR/mor-tier.cha" << 'FRAG'
%mor:	v|want qn|more n|cookie .
FRAG

cat > "$FRAGMENTS_DIR/gra-tier.cha" << 'FRAG'
%gra:	1|2|SUBJ 2|0|ROOT 3|2|OBJ 4|2|PUNCT
FRAG

cat > "$FRAGMENTS_DIR/participants-header.cha" << 'FRAG'
@Participants:	CHI Target_Child, MOT Mother, FAT Father
FRAG

cat > "$FRAGMENTS_DIR/date-header.cha" << 'FRAG'
@Date:	15-JAN-2024
FRAG

cat > "$FRAGMENTS_DIR/languages-header.cha" << 'FRAG'
@Languages:	eng, fra
FRAG

cat > "$FRAGMENTS_DIR/utterance-block.cha" << 'FRAG'
*CHI:	hello world .
%mor:	co|hello n|world .
%gra:	1|2|LINK 2|0|ROOT 3|2|PUNCT
FRAG

cat > "$FRAGMENTS_DIR/full-document.cha" << 'FRAG'
@UTF8
@Begin
@Participants:	CHI Target_Child
*CHI:	hi .
@End
FRAG

echo "=== Fragment Parsing Tests ==="
cd "$VARIANT_DIR"

passed=0
failed=0
total=0

for frag in "$FRAGMENTS_DIR"/*.cha; do
    fname=$(basename "$frag" .cha)
    total=$((total + 1))

    echo ""
    echo "--- $fname ---"
    echo "Input: $(cat "$frag")"

    # Parse and show the tree
    result=$(tree-sitter parse "$frag" 2>&1)
    echo "Tree:"
    echo "$result" | head -20

    # Check for ERROR nodes
    if echo "$result" | grep -q '(ERROR'; then
        echo "RESULT: PARTIAL (has ERROR nodes)"
        failed=$((failed + 1))
    elif echo "$result" | grep -q '(MISSING'; then
        echo "RESULT: PARTIAL (has MISSING nodes)"
        failed=$((failed + 1))
    else
        echo "RESULT: CLEAN"
        passed=$((passed + 1))
    fi
done

echo ""
echo "=== Summary ==="
echo "Total: $total, Clean: $passed, Partial: $failed"
echo "$passed/$total clean" > "$RESULTS_DIR/fragment-test-results.txt"
