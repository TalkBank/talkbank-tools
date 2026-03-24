#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ALLOWLIST_FILE="$ROOT_DIR/scripts/errorsink_option_allowlist.txt"

if [[ ! -f "$ALLOWLIST_FILE" ]]; then
  echo "Missing allowlist: $ALLOWLIST_FILE" >&2
  exit 1
fi

TMP_CURRENT="$(mktemp)"
TMP_ALLOWLIST="$(mktemp)"
trap 'rm -f "$TMP_CURRENT" "$TMP_ALLOWLIST"' EXIT

cd "$ROOT_DIR"

FILE_LIST="$(
  find \
    crates/talkbank-parser/src \
    -name '*.rs' -type f
)"

while IFS= read -r file; do
  [[ -n "$file" ]] || continue
  perl -0777 -ne '
while (/fn\s+([A-Za-z0-9_]+)(?:\s*<[^>\n]*>)?\s*\(([^)]*)\)\s*->\s*Option</sg) {
  my ($name, $args) = ($1, $2);
  if ($args =~ /errors:\s*&impl\s+ErrorSink/s) {
    print "$ARGV:fn $name\n";
  }
}
' "$file"
done <<< "$FILE_LIST" | sort -u > "$TMP_CURRENT"

sort "$ALLOWLIST_FILE" > "$TMP_ALLOWLIST"

NEW_MATCHES="$(comm -13 "$TMP_ALLOWLIST" "$TMP_CURRENT")"
REMOVED_MATCHES="$(comm -23 "$TMP_ALLOWLIST" "$TMP_CURRENT")"

if [[ -n "$NEW_MATCHES" ]]; then
  echo "New 'ErrorSink + Option' parser signatures detected." >&2
  echo "These should use explicit outcome types instead of Option:" >&2
  echo "$NEW_MATCHES" >&2
  exit 1
fi

if [[ -n "$REMOVED_MATCHES" ]]; then
  echo "Info: some allowlisted signatures were removed; consider updating allowlist:"
  echo "$REMOVED_MATCHES"
fi

COUNT="$(wc -l < "$TMP_CURRENT" | tr -d ' ')"
echo "Guardrail check passed: $COUNT allowlisted 'ErrorSink + Option' parser signatures."
