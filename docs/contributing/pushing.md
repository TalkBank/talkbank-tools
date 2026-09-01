# Pushing without CI churn

**Status:** Current
**Last updated:** 2026-09-01 13:18 EDT

Three pushes to `main` on 2026-08-14 each turned CI red and each needed a
follow-up commit. The pre-push hook reported "All pre-push checks passed" every
time. This is what changed so that stops, and the one case that local checks
can never cover.

## The hook must actually be installed, and it chains to a local hook

`git` runs exactly one file, `.git/hooks/pre-push`, and that directory is
unversioned. Anything else that writes it (another tool's installer, a hand
symlink) displaces this gate WITHOUT SAYING SO, and the next push reports only
whatever the displacing hook checks. That happened between 2026-08-19 and
2026-09-01: a separate local screen owned `.git/hooks/pre-push`, so two
release-prep pushes reached `main` that `make batchalign-ci-rust` refuses (a
version bump that missed a generated artifact). The gate was fine; it was not
running.

Two consequences. `make install-hooks` is the thing to re-run after anything
touches `.git/hooks`, and `scripts/pre-push.sh` ends by chaining to
`.git/hooks/pre-push.local` if that file is executable, so a further local
check coexists with the gate instead of replacing it. The gate runs first; the
local hook receives git's ref list unchanged.

## The hook runs CI's target, not a copy of it

`scripts/pre-push.sh` invokes `make batchalign-ci-rust`, the same target
`.github/workflows/batchalign-rust.yml` invokes. It used to run a hand-written
subset (`fmt`, an affected-only compile check, and clippy only when
`TALKBANK_PRE_PUSH_CLIPPY=1`, which defaulted to off) while claiming in its own
docstring to "catch anything the GitHub main CI workflow would flag".

Two lists of what must pass will drift. `scripts/check_push_gate_sync.py` now
fails if a `make` target a push-triggered workflow runs is not reached by the
hook, and it runs inside `make lint`, so the loop closes: the hook runs the CI
target, which checks that the hook runs the CI target.

The subset existed to keep the hook fast. Measured on a warm tree, the full
target is **16 seconds**. There was nothing to save.

Two things about that checker are worth knowing, because both were wrong on its
first day and each let a red build through:

- **Coverage is computed through the Makefile, not by comparing two texts.** A
  hook line reading `make batchalign-ci-rust` covers everything that recipe
  invokes, transitively. Demanding the hook restate the recipe would be the same
  mirroring one level down.
- **It reads every push-triggered workflow**, decided by each workflow's own
  `on:` triggers. It originally read only the file mentioning
  `batchalign-ci-rust`, so the Python job was outside the scan entirely and a
  `ruff format --check` failure reached `main`. That job now calls
  `make batchalign-lint-python-source`, which the hook also runs: a workflow
  step that invokes a tool directly instead of through a target is invisible to
  a checker that reads targets.

`EXEMPT` in that file is the honest statement of what a green hook does **not**
prove. Everything listed there needs a maturin release wheel, the
npm-installed frontend, or the built binary, so it costs minutes and is covered
only once CI runs.

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

## A gate must test the commit, not the day

The fourth red run that afternoon was different in kind from the other three,
and it is the one worth remembering. Nothing in the push was wrong: an advisory
had been published against a transitive dependency since the previous run, and
`pip-audit` exits non-zero on any advisory outside its allowlist.

No local hook can catch that, because there is nothing to catch. The commit did
not change. And the fix available to the person blocked is to append another
identifier to an allowlist, which is not a fix, it is the gate asking to be
turned off one line at a time. The allowlist had reached five entries, every one
of them recording the same verdict (this code loads only pinned first-party
models, so the vulnerable path is unreachable), and the sixth would have said it
again on a deploy day, for a vulnerability whose upstream fix exists only as an
unreleased commit.

So dependency advisories moved to `.github/workflows/dependency-audit.yml`,
weekly plus `workflow_dispatch`. The information is unchanged and Dependabot
still opens PRs for anything with a real fix; what is gone is the blocking.

**The test to apply before adding anything to `ci.yml`:** can a developer make
this pass before pushing? If the answer depends on the state of the world rather
than the state of the commit, it is a report, not a gate, and it belongs on a
schedule.

## Doctests are a separate compilation

`cargo test --lib` and `cargo clippy --all-targets` do not see them.
`make batchalign-ci-rust` does, via `batchalign-test-rust`. The third failure
was a doctest still passing `&["cha"]` to a function whose parameter had become
a typed `InputKind`, which no `--lib` or `--all-targets` run could have caught.
