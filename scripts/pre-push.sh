#!/usr/bin/env bash
# Pre-push hook: fast local checks that mirror CI gates.
# Install: make install-hooks
#
# Coverage goal: catch anything the GitHub "main CI" workflow would flag
# on a push to main, without running long test suites. If a CI job can
# fail purely because of committed content (not runtime behavior), this
# hook must cover it.
set -euo pipefail

echo "==> pre-push: fmt check"
cargo fmt --all -- --check

echo "==> pre-push: affected compile check"
cargo run -q -p xtask -- affected-rust check

# The CHAT-format pre-push gates (spec/tools fmt, parser signature guardrail,
# generated-artifacts check, fuzz workspace isolation) moved to chatter, which
# is now the single home for the CHAT core. This hook guards the batchalign
# layer talkbank-tools still owns.

# Mirrors the "TalkBank Toolchain mdBook" CI workflow. mdbook's
# linkcheck2 backend exhaustively verifies every relative link
# against SUMMARY.md, catching SUMMARY-unreachable targets like
# the 2026-05-22 batchalign/introduction.md regression that broke
# CI after a 68-commit squash push. Requires mdbook + mdbook-
# linkcheck + mdbook-mermaid on PATH (make book-check enforces).
echo "==> pre-push: mdBook build + linkcheck"
make book-check

if [[ "${TALKBANK_PRE_PUSH_CLIPPY:-0}" == "1" ]]; then
  echo "==> pre-push: affected clippy"
  cargo run -q -p xtask -- affected-rust clippy
fi

echo "✓ All pre-push checks passed"
