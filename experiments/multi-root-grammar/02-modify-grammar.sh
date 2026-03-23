#!/bin/bash
# 02-modify-grammar.sh — Create multi-root grammar variant
set -euo pipefail

GRAMMAR_DIR="$(cd "$(dirname "$0")/../../grammar" && pwd)"
EXPERIMENT_DIR="$(dirname "$0")"
VARIANT_DIR="$EXPERIMENT_DIR/variant"

# Copy the full grammar directory
echo "=== Creating multi-root variant ==="
rm -rf "$VARIANT_DIR"
cp -r "$GRAMMAR_DIR" "$VARIANT_DIR"

# Remove generated artifacts (we'll regenerate)
rm -f "$VARIANT_DIR/src/parser.c"

# Apply the multi-root transformation using Python
python3 - "$VARIANT_DIR" << 'PYEOF'
import re
import sys

variant_dir = sys.argv[1]
grammar_path = f"{variant_dir}/grammar.js"

with open(grammar_path) as f:
    source = f.read()

# Strategy: rename the existing `document` rule to `full_document`,
# then create a new `document` rule that is a choice of entry points.
#
# The existing document rule looks like:
#   document: $ => seq($.utf8_header, ...)
#
# We rename it and add a new root.

# Step 1: Find and rename the document rule
# The rule is:  document: $ => seq(...)
# We change it to: full_document: $ => seq(...)
modified = source.replace(
    "document: $ => seq(",
    "full_document: $ => seq(",
    1  # only first occurrence
)

# Step 2: Add new document rule at the beginning of the rules block
# Find the "rules:" line and insert after it
new_root = '''
    // Multi-root: parse full documents OR individual fragments.
    // Precedence: full_document > utterance > individual lines.
    // This resolves ambiguity when a main_tier could be either
    // a standalone fragment or part of an utterance.
    document: $ => choice(
      prec(3, $.full_document),    // complete @UTF8...@Begin...@End file
      prec(2, $.utterance),        // main tier + dependent tiers
      prec(1, $.main_tier),        // single *SPEAKER:\\tcontent terminator
      prec(1, $.dependent_tier),   // single %tier:\\tcontent (any tier type)
      prec(1, $.header),           // single @Header:\\tcontent
      prec(1, $.pre_begin_header), // @PID, @Font, @Window, @Color words
    ),

'''

# Insert after "rules: $ => ({" or "rules: {"
# The rules block in CHAT grammar.js starts with:
#   rules: {
# We need to insert after the opening of the rules object

# Find the pattern "full_document:" (our renamed rule) and insert before it
modified = modified.replace(
    "full_document: $ => seq(",
    new_root + "    full_document: $ => seq(",
    1
)

# Step 3: Update any internal references to $.document
# (there shouldn't be any since document is the root, but check)
# Actually, we should NOT replace $.document references — those don't exist
# in typical grammars since document is the root and never self-referenced.

# Step 4: Add conflict declaration for document vs utterance
# Find the conflicts array and add our new conflict
if 'conflicts: $ => [' in modified:
    modified = modified.replace(
        'conflicts: $ => [',
        'conflicts: $ => [\n      [$.document, $.utterance],',
        1
    )
elif 'conflicts: [' in modified:
    modified = modified.replace(
        'conflicts: [',
        'conflicts: [\n      [$.document, $.utterance],',
        1
    )

with open(grammar_path, 'w') as f:
    f.write(modified)

print(f"Multi-root grammar written to {grammar_path}")
print("Changes:")
print("  1. Renamed 'document' → 'full_document'")
print("  2. Added new 'document' as choice(full_document, utterance, main_tier, ...)")
PYEOF

echo ""
echo "Variant created in $VARIANT_DIR/"
echo "Next: run 03-generate.sh to compile the variant"
