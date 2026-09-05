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

# CI may supply the artifact it built from the checked-out commit. Locally,
# ask Cargo to prove target/debug is current. Cargo's dependency graph is the
# source-of-truth freshness check; comparing mtimes by hand previously accepted
# stale binaries and also triggered an unrelated release build on a cold tree.
if [[ -n "${BATCHALIGN_BIN:-}" && -x "${BATCHALIGN_BIN}" ]]; then
    exec "$BATCHALIGN_BIN" ipc-schema --check --output ipc-schema/
fi

cargo build -q -p batchalign
exec ./target/debug/batchalign3 ipc-schema --check --output ipc-schema/
