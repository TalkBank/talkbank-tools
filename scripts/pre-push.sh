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

# The hygiene jobs CI runs in their OWN jobs. They are separate targets rather
# than prerequisites of `lint` because CI invokes `lint` (via
# `batchalign-ci-rust`) on a cargo-only runner: adding shellcheck or actionlint
# there kills that job with `Error 127`, which is exactly what happened on
# 2026-08-25 when they were briefly folded in.
echo "==> pre-push: shellcheck (CI's 'Shell scripts' job)"
make lint-shell
echo "==> pre-push: actionlint (CI's 'Workflow files' job)"
make lint-actionlint

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

echo "==> pre-push: IPC schema drift"
# `batchalign-ipc-schema-check` asks Cargo to prove a development binary is
# current, then runs that exact binary. On a warm tree this is a few seconds;
# on a cold tree it links development mode rather than compiling an unrelated
# optimized release artifact.
#
# It earned its place on 2026-08-16. `numeric_id!` derives
# `schemars::JsonSchema`, which copies a Rust doc comment into the schema's
# `description`, so editing the docstring on `DurationMs` silently regenerated
# six files under `ipc-schema/worker_v2/` and CI rejected a push that every
# local gate had passed. Nothing about that change looked like it touched a
# wire format.
make batchalign-ipc-schema-check

# `openapi.json` is GENERATED from the Rust types, so a doc-comment or a
# `required` change makes the committed copy stale. The FULL dashboard check
# also runs `npx openapi-typescript` and stays exempt for needing the
# npm-installed frontend; this half reuses the binary the IPC check just proved
# current. A stale openapi.json reached main on
# 2026-08-27 through exactly that exemption.
echo "==> pre-push: generated openapi.json drift"
# The preceding IPC check asked Cargo to prove this exact development binary is
# current. Reuse it instead of asking Cargo to walk the same dependency graph a
# second time.
BATCHALIGN_BIN="$PWD/target/debug/batchalign3" make batchalign-dashboard-schema-check

echo "==> pre-push: mdBook build + linkcheck"
# Not part of batchalign-ci-rust: it is a separate workflow. linkcheck2 verifies
# every relative link against SUMMARY.md, which is how a SUMMARY-unreachable
# page broke CI after a 68-commit squash push in May.
make book-check

echo "✓ pre-push ran CI's own target; anything it missed is platform-conditional"

# Chain to an untracked local hook, if one is installed. `.git/hooks` is not
# versioned, so a contributor may keep a further pre-push check of their own at
# `pre-push.local`; git runs only THIS file, so this is the only way that check
# runs at all. Order matters: the gate above first, then the local hook. This
# hook never reads stdin, so the ref list git provides reaches the local hook
# untouched. Keep this free of any particular local hook's identity: what a
# contributor checks on their own machine is theirs.
LOCAL_HOOK="$(git rev-parse --git-path hooks)/pre-push.local"
if [[ -x "$LOCAL_HOOK" ]]; then
    exec "$LOCAL_HOOK" "$@"
fi
