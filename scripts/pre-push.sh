#!/usr/bin/env bash
# Pre-push hook: runs the SAME target CI runs. Install: make install-hooks
#
# It does not mirror CI, it invokes CI's target. A mirror is a second list of
# what must pass, and a second list drifts: this hook used to run `fmt`, an
# `affected-rust check`, and clippy only when `TALKBANK_PRE_PUSH_CLIPPY=1`
# (default 0), while CI ran `make batchalign-ci-rust`. Its own docstring
# promised to "catch anything the GitHub main CI workflow would flag on a push
# to main", and on 2026-08-14 it printed "All pre-push checks passed" three
# times for pushes CI then rejected.
#
# The justification for the weaker subset was speed. Measured on a warm tree,
# the full target is 16 SECONDS, so there was nothing to save.
#
# WHAT THIS CANNOT CATCH, and it is the reason for the branch flow below:
# CI runs on Linux and developers run macOS. Two of those three failures were
# `cfg(target_os)`-conditional, and no macOS command can see them:
#   - the Tauri desktop crate needs glib, absent on the runner, so a
#     `--workspace` clippy passes here and fails there;
#   - a helper used only inside a `cfg(target_os = "macos")` block is live
#     here and dead code there.
# For those, the ONLY gate is CI itself, which is why main should receive a
# commit CI has already seen. See `docs/contributing/pushing.md`.
set -euo pipefail

echo "==> pre-push: the CI target (make batchalign-ci-rust)"
make batchalign-ci-rust

echo "==> pre-push: ruff lint + format over the Python sources"
# The SOURCE-only half of the Python gate: ruff reads the tree, so this needs
# no wheel and takes about a second. The rest of that job (mypy, the drift
# checks) needs a maturin release build and is listed in EXEMPT in
# check_push_gate_sync.py with its reason. A `ruff format --check` failure
# reached main on 2026-08-14 because CI ran the tool directly instead of a make
# target, which made it invisible to the coverage check.
make batchalign-lint-python-source

echo "==> pre-push: mdBook build + linkcheck"
# Not part of batchalign-ci-rust: it is a separate workflow. linkcheck2 verifies
# every relative link against SUMMARY.md, which is how a SUMMARY-unreachable
# page broke CI after a 68-commit squash push in May.
make book-check

echo "✓ pre-push ran CI's own target; anything it missed is platform-conditional"
