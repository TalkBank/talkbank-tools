#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_CHAT_HTML_URL="https://talkbank.org/0info/manuals/CHAT.html"
CHAT_HTML_PATH="${1:-${CHAT_HTML_PATH:-}}"
CHAT_HTML_URL="${CHAT_HTML_URL:-$DEFAULT_CHAT_HTML_URL}"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

if [[ -z "${CHAT_HTML_PATH}" || ! -f "$CHAT_HTML_PATH" ]]; then
  if command -v curl >/dev/null 2>&1; then
    if [[ -n "${CHAT_HTML_PATH}" ]]; then
      echo "Local CHAT.html not found at: $CHAT_HTML_PATH" >&2
    else
      echo "CHAT_HTML_PATH not provided." >&2
    fi
    echo "Downloading CHAT manual from: $CHAT_HTML_URL" >&2
    CHAT_HTML_PATH="$tmpdir/CHAT.html"
    # --retry-all-errors covers transient DNS and connect failures on CI runners.
    curl -fsSL --retry 3 --retry-delay 5 --retry-all-errors --connect-timeout 15 \
      "$CHAT_HTML_URL" -o "$CHAT_HTML_PATH"
  else
    cat >&2 <<EOF
ERROR: CHAT manual HTML not found locally and curl is unavailable.

CHAT_HTML_PATH:
  ${CHAT_HTML_PATH:-"(not provided)"}

CHAT_HTML_URL:
  $CHAT_HTML_URL

Usage:
  scripts/check-chat-manual-anchors.sh /abs/path/to/CHAT.html
or:
  CHAT_HTML_PATH=/abs/path/to/CHAT.html make chat-anchors-check
or:
  CHAT_HTML_URL=https://talkbank.org/0info/manuals/CHAT.html make chat-anchors-check
EOF
    exit 2
  fi
fi

refs_file="$tmpdir/referenced_anchors.txt"
html_file="$tmpdir/html_anchors.txt"
missing_file="$tmpdir/missing_anchors.txt"

# Source/doc surfaces with CHAT links used by contributors and generated schema docs.
scan_paths=(
  "$ROOT_DIR/crates"
  "$ROOT_DIR/schema"
  "$ROOT_DIR/docs"
)

if command -v rg >/dev/null 2>&1; then
  rg -o 'CHAT\.html#[A-Za-z0-9_]+' "${scan_paths[@]}" \
    | sed 's/.*#//' \
    | sort -u > "$refs_file" || true
else
  grep -RhoE 'CHAT\.html#[A-Za-z0-9_]+' "${scan_paths[@]}" \
    | sed 's/.*#//' \
    | sort -u > "$refs_file" || true
fi

if command -v rg >/dev/null 2>&1; then
  {
    rg -o 'id="[A-Za-z0-9_:-]+"' "$CHAT_HTML_PATH" | sed -E 's/id="([^"]+)"/\1/'
    rg -o 'name="[A-Za-z0-9_:-]+"' "$CHAT_HTML_PATH" | sed -E 's/name="([^"]+)"/\1/'
    rg -o 'name=[A-Za-z0-9_:-]+' "$CHAT_HTML_PATH" | sed -E 's/name=([A-Za-z0-9_:-]+)/\1/'
  } | sort -u > "$html_file"
else
  {
    grep -oE 'id="[A-Za-z0-9_:-]+"' "$CHAT_HTML_PATH" | sed -E 's/id="([^"]+)"/\1/'
    grep -oE 'name="[A-Za-z0-9_:-]+"' "$CHAT_HTML_PATH" | sed -E 's/name="([^"]+)"/\1/'
    grep -oE 'name=[A-Za-z0-9_:-]+' "$CHAT_HTML_PATH" | sed -E 's/name=([A-Za-z0-9_:-]+)/\1/'
  } | sort -u > "$html_file"
fi

comm -23 "$refs_file" "$html_file" > "$missing_file"

echo "referenced_anchors=$(wc -l < "$refs_file")"
echo "chat_html_anchors=$(wc -l < "$html_file")"
echo "missing=$(wc -l < "$missing_file")"
echo "chat_html=$CHAT_HTML_PATH"

if [[ -s "$missing_file" ]]; then
  echo "Missing anchor ids:"
  sed 's/^/- /' "$missing_file"
  exit 1
fi

echo "All referenced CHAT anchors resolve."
