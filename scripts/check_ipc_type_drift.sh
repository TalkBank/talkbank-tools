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

# Use an already-built binary when there is one. `cargo run` would compile the
# whole crate in the DEBUG profile, which shares no artifacts with the release
# build the wheel and the deploy use, so running this gate after a normal build
# used to pay for a second full compile.
if [[ -n "${BATCHALIGN_BIN:-}" && -x "${BATCHALIGN_BIN}" ]]; then
    exec "$BATCHALIGN_BIN" ipc-schema --check --output ipc-schema/
fi

if [[ -x "target/release/batchalign3" ]]; then
    exec ./target/release/batchalign3 ipc-schema --check --output ipc-schema/
fi

if command -v batchalign3 >/dev/null 2>&1; then
    exec batchalign3 ipc-schema --check --output ipc-schema/
fi

exec cargo run --release -q -p batchalign -- ipc-schema --check --output ipc-schema/
