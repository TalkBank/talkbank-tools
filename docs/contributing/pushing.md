# Pushing without CI churn

**Status:** Current
**Last updated:** 2026-08-14

Three pushes to `main` on 2026-08-14 each turned CI red and each needed a
follow-up commit. The pre-push hook reported "All pre-push checks passed" every
time. This is what changed so that stops, and the one case that local checks
can never cover.

## The hook runs CI's target, not a copy of it

`scripts/pre-push.sh` invokes `make batchalign-ci-rust`, the same target
`.github/workflows/batchalign-rust.yml` invokes. It used to run a hand-written
subset (`fmt`, an affected-only compile check, and clippy only when
`TALKBANK_PRE_PUSH_CLIPPY=1`, which defaulted to off) while claiming in its own
docstring to "catch anything the GitHub main CI workflow would flag".

Two lists of what must pass will drift. `scripts/check_push_gate_sync.py` now
fails if a `make` target the workflow runs is absent from the hook, and it runs
inside `make lint`, so the loop closes: the hook runs the CI target, which
checks that the hook runs the CI target.

The subset existed to keep the hook fast. Measured on a warm tree, the full
target is **16 seconds**. There was nothing to save.

## What local checks cannot catch: the runner is Linux

CI runs `ubuntu-latest`; development happens on macOS. Anything behind
`cfg(target_os)` is invisible to every local command, and two of the three
failures were exactly that:

- The Tauri desktop crate depends on `glib` on Linux and not on macOS, so
  `cargo clippy --workspace` passed locally and failed on the runner. (Fixed by
  scoping the lint to the crates CI gates, which is why every other CI target
  in the Makefile enumerates crates instead of using `--workspace`.)
- `dashboard_auto_open_enabled` is called only inside a
  `cfg(target_os = "macos")` block, so it is live locally and dead code on the
  runner, which `-D warnings` rejects.

**No amount of local gating catches these.** If you want to reproduce one, flip
the target conditions in the file and run clippy: changing `target_os = "macos"`
to `"linux"` makes the macOS branch inactive and reproduces the runner's view.
That is how the second failure above was diagnosed and its fix verified.

## So: let CI see it before `main` does

For any change that touches platform-conditional code, build configuration,
lint configuration, or the workspace's crate set:

1. Push the work to a branch, not `main`.
2. Let the Rust workflow run.
3. Fast-forward `main` only once it is green.

`main` then only ever receives commits CI has already validated, which is what
keeps its history free of "fix CI" commits. For ordinary changes to crate
internals, the hook is sufficient and a direct push to `main` is fine.

## Doctests are a separate compilation

`cargo test --lib` and `cargo clippy --all-targets` do not see them.
`make batchalign-ci-rust` does, via `batchalign-test-rust`. The third failure
was a doctest still passing `&["cha"]` to a function whose parameter had become
a typed `InputKind`, which no `--lib` or `--all-targets` run could have caught.
