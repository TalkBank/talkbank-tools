#!/usr/bin/env bash
# Parity-test one CLAN command against chatter clan <cmd>.
#
# Usage:
#   scripts/clan-parity/run-parity.sh <cmd> <fixture.cha> [extra_args...]
#
# Writes ./out/<cmd>.{clan,chatter}.txt and a side-by-side diff.
#
# OSX-CLAN source convention: most CLAN analysis commands read input via
# stdin redirection (`<`). There is no positional file argument. See the
# OSX-CLAN/src/clan/<cmd>.cpp usage() function for the exact flag set.
#
# Build dependency: OSX-CLAN binaries at OSX-CLAN/src/unix/bin/. Rebuild
# with:
#   cd OSX-CLAN/src/clan
#   make -B CC=g++ CFLAGS='-O -DUNX -DAPPLEUNX -fshort-wchar -Wcomment \
#     -Wno-deprecated-declarations -Wempty-body \
#     -Wno-c++11-compat-deprecated-writable-strings' <cmd>
set -euo pipefail
cmd="${1:-}"
fixture="${2:-}"
shift 2 || true
if [[ -z "$cmd" || -z "$fixture" ]]; then
  echo "usage: $0 <cmd> <fixture.cha> [extra_args...]" >&2
  exit 2
fi

WORKSPACE="$(cd "$(dirname "$0")/../.." && pwd)"
CLAN_BIN="$WORKSPACE/../OSX-CLAN/src/unix/bin/$cmd"
mkdir -p "$WORKSPACE/scripts/clan-parity/out"
clan_out="$WORKSPACE/scripts/clan-parity/out/$cmd.clan.txt"
chatter_out="$WORKSPACE/scripts/clan-parity/out/$cmd.chatter.txt"

if [[ ! -x "$CLAN_BIN" ]]; then
  echo "OSX-CLAN binary missing: $CLAN_BIN" >&2
  echo "Build with the make command above." >&2
  exit 3
fi

# CLAN: stdin redirection
"$CLAN_BIN" "$@" < "$fixture" > "$clan_out" 2>&1 || true
# chatter: positional file arg
cargo run --release -q -p talkbank-cli -- clan "$cmd" "$@" "$fixture" > "$chatter_out" 2>&1 || true

echo "=== CLAN ($CLAN_BIN) ==="
cat "$clan_out"
echo
echo "=== chatter clan $cmd ==="
cat "$chatter_out"
echo
echo "=== diff CLAN -> chatter ==="
diff -u "$clan_out" "$chatter_out" || true
