#!/usr/bin/env bash
# Pre-push hook: fast local checks that mirror CI gates.
# Install: make install-hooks
set -euo pipefail

echo "==> pre-push: fmt check"
cargo fmt --all -- --check
cd spec/tools && cargo fmt --all -- --check && cd ../..

echo "==> pre-push: clippy"
cargo clippy --all-targets -- -D warnings

echo "==> pre-push: compile check"
cargo check --workspace --all-targets
cd spec/tools && cargo check --all-targets && cd ../..

echo "==> pre-push: parser guardrail"
scripts/check-errorsink-option-signatures.sh

echo "✓ All pre-push checks passed"
