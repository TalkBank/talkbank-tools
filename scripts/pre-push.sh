#!/usr/bin/env bash
# Pre-push hook: fast local checks that mirror CI gates.
# Install: make install-hooks
set -euo pipefail

echo "==> pre-push: fmt check"
cargo fmt --all -- --check
cd spec/tools && cargo fmt --all -- --check && cd ../..

echo "==> pre-push: affected compile check"
cargo run -q -p xtask -- affected-rust check

echo "==> pre-push: parser guardrail"
scripts/check-errorsink-option-signatures.sh

if [[ "${TALKBANK_PRE_PUSH_CLIPPY:-0}" == "1" ]]; then
  echo "==> pre-push: affected clippy"
  cargo run -q -p xtask -- affected-rust clippy
fi

echo "✓ All pre-push checks passed"
