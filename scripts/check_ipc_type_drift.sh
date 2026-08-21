#!/usr/bin/env bash
# Check that the Rust IPC types and the committed JSON Schema are in sync.
#
# Exits non-zero if any schema file is stale, missing, or orphaned.
# Run after modifying Rust types that cross the Python boundary.
#
# Usage: bash scripts/check_ipc_type_drift.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Newest tracked Rust source the schema is derived from. Used to decide whether
# a prebuilt binary is still allowed to answer this question; see below.
newest_source_epoch() {
    local newest=0 epoch source
    while IFS= read -r source; do
        [ -e "$source" ] || continue
        epoch=$(stat -f %m "$source")
        if [ "$epoch" -gt "$newest" ]; then
            newest=$epoch
        fi
    done < <(git ls-files 'crates/*.rs')
    printf '%s\n' "$newest"
}

SOURCE_EPOCH="$(newest_source_epoch)"

# A prebuilt binary may answer only if it is NEWER than every Rust source.
#
# Reusing an existing binary is a deliberate speed optimization: `cargo run`
# would compile the whole crate in the DEBUG profile, which shares no artifacts
# with the release build the wheel and the deploy use, so this gate used to pay
# for a second full compile.
#
# But the optimization silently traded correctness for speed. `generate_ipc_types.sh`
# always runs `cargo run` (current source), while this script preferred whatever
# binary happened to be lying around. When the two disagreed, the gate reported
# on a build rather than on the tree, in BOTH directions: a stale binary invents
# drift against a correct schema, and, far worse, it reports "up to date" while
# real drift sits in the source. On 2026-08-21 both happened within an hour.
is_fresh() {
    local candidate=$1 epoch
    epoch=$(stat -f %m "$candidate")
    [ "$epoch" -ge "$SOURCE_EPOCH" ]
}

if [[ -n "${BATCHALIGN_BIN:-}" && -x "${BATCHALIGN_BIN}" ]]; then
    if is_fresh "$BATCHALIGN_BIN"; then
        exec "$BATCHALIGN_BIN" ipc-schema --check --output ipc-schema/
    fi
    echo "note: BATCHALIGN_BIN is older than the Rust sources; rebuilding instead" >&2
fi

if [[ -x "target/release/batchalign3" ]] && is_fresh "target/release/batchalign3"; then
    exec ./target/release/batchalign3 ipc-schema --check --output ipc-schema/
fi

if command -v batchalign3 >/dev/null 2>&1; then
    resolved=$(command -v batchalign3)
    if is_fresh "$resolved"; then
        exec batchalign3 ipc-schema --check --output ipc-schema/
    fi
fi

exec cargo run --release -q -p batchalign -- ipc-schema --check --output ipc-schema/
