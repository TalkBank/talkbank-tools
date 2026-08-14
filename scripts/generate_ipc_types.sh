#!/usr/bin/env bash
# Regenerate the JSON Schema that describes the Rust IPC types.
#
# Pipeline: Rust (schemars) -> JSON Schema (ipc-schema/)
#
# Python models are HAND-WRITTEN and checked against this schema by
# `batchalign/tests/test_ipc_type_conformance.py`. That they are not generated
# is deliberate; the reasons are on the "Rust to Python IPC Type Sync"
# developer page, under "Why the Python models are hand-written".
#
# Usage: bash scripts/generate_ipc_types.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "Generating JSON Schema from Rust types..."
cargo run -q -p batchalign -- ipc-schema --output ipc-schema/

echo "Done. Run 'bash scripts/check_ipc_type_drift.sh' to verify, and"
echo "'uv run pytest batchalign/tests/test_ipc_type_conformance.py' to check"
echo "the hand-written Python models against the result."
