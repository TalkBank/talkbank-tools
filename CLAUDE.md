# CLAUDE.md

**Last modified:** 2026-07-30 10:37 EDT

Guidance for Claude Code (claude.ai/code) when working in the `talkbank-tools`
repository.

## What this repo is now (read first)

`talkbank-tools` is the **batchalign3 workspace**: the Batchalign ML pipeline
(ASR, forced alignment, neural morphotag, utterance segmentation), its PyO3
bridge, Python package, dashboard, and experimental desktop shell. **It is no
longer a CHAT-format toolchain.** The CHAT core (grammar, spec, tree-sitter
parser, data model, validation, transform, CLI, LSP, CLAN) lives wholly in the
**chatter** repo (`TalkBank/chatter`, sibling clone at `../chatter`), which is
the single home for the CHAT format. This workspace **consumes** chatter's
crates.

History: chatter was extracted from talkbank-tools in 2026-05/06; the duplicate
CHAT core was then removed from talkbank-tools and batchalign repointed at
chatter on 2026-06-18.

### How the CHAT core is consumed

`[workspace.dependencies]` in `Cargo.toml` points `talkbank-model`,
`talkbank-parser`, `talkbank-parser-re2c`, `talkbank-parser-tests`, and
`talkbank-transform` at the published, public chatter via git deps pinned to a
release tag (see `Cargo.toml` for the current tag; e.g. `{ git = "https://github.com/TalkBank/chatter", tag = "v0.3.5" }`).
A plain checkout builds with no `../chatter` sibling. Adopt a newer chatter by
bumping the tag (then `cargo update`). **Do NOT re-introduce copies of those
crates here**; chatter owns them. New CHAT-format / grammar / spec / parser /
validation / CLAN work goes in chatter, not here.

To co-develop chatter+batchalign locally, add an UNCOMMITTED `[patch]` that
points the git deps at your local checkout (never commit it; committed builds
stay self-contained):

```toml
[patch."https://github.com/TalkBank/chatter"]
talkbank-model = { path = "../chatter/crates/talkbank-model" }
talkbank-parser = { path = "../chatter/crates/talkbank-parser" }
talkbank-parser-re2c = { path = "../chatter/crates/talkbank-parser-re2c" }
talkbank-parser-tests = { path = "../chatter/crates/talkbank-parser-tests" }
talkbank-transform = { path = "../chatter/crates/talkbank-transform" }
```

## Crates in this workspace

| Crate | Purpose |
|-------|---------|
| `batchalign` | The Batchalign pipeline: ASR, FA, morphotag, jobs/runner, store, dashboard API |
| `batchalign-transform` | Batchalign-specific CHAT transforms (`asr_postprocess`, `morphosyntax`, `utseg`, FA `decisions`, `compare`, `build_chat`, `dp_align`, ...) layered over chatter's generic `talkbank-transform`, which it re-exports via a facade (`pub use talkbank_transform::*`) |
| `batchalign-pyo3` | PyO3 bridge for the Python package |
| `batchalign-types`, `batchalign-whisper-pilot` (experimental) | Shared types |

Plus `apps/dashboard-desktop` (Tauri shell, experimental, excluded from CI
gates), `frontend/` (React dashboard), the `batchalign` / `batchalign_core`
Python packages, and `xtask` (build helpers).

## Crate boundary

The `batchalign-*` crates are the ML application; they **consume** chatter's
`talkbank-*` crates and never reimplement CHAT primitives. A CHAT primitive has
one home (chatter). Decision test for new code: if it fundamentally needs ML
models, audio/signal processing, network services, or fleet runtime, it belongs
here; otherwise it belongs in chatter.

## Build, Test, Lint

`make help` lists the targets; CI workflows live in
`.github/workflows/`. Shell scripts pass `shellcheck` at default
severity (`scripts/lint/shellcheck-all.sh`; wired into the ci-report
gate). Rust tests: prefer scoped `cargo test -p <crate>` (never
`cargo nextest`, which is banned and uninstalled in this workspace).
Developer procedure pages: `book/src/batchalign/developer/`.

## Releases and Versioning

No release semver is baked in: `batchalign3 version` reports a
`git describe`-based BUILD identity assembled in
`crates/batchalign/build.rs`; staleness is judged by build identity,
never semver. Releases are GitHub Releases (uv-bootstrap installer +
abi3 wheels), **never PyPI**. Release workflow:
`.github/workflows/batchalign-release.yml` (tag push or
workflow_dispatch with dry_run).

## Cross-Cutting Design Rules

1. **Types are the first layer of documentation.** Prefer named structs, enums,
   traits, and newtypes over raw primitives when a value has stable meaning.
2. **No primitive obsession at stable boundaries.** No raw strings/ints/bools for
   domain concepts (timestamps, language IDs, spans, indices, counts, engine
   selections, job/file states).
3. **No tuple-packed domain seams.** Name pairs/tuples with a struct or newtype.
4. **Avoid boolean blindness.** Use enums or state types for multiple meaningful
   states; no `tui`/`no_tui`-style bool pairs.
5. **No panic-based control flow in long-lived logic.** No `unwrap()`/`expect()`
   in pipeline, runner, store, FFI, or background paths that should report typed
   failures.
6. **Use real domain errors** (`thiserror`), not stringly failures.
7. **Keep modules browseable.** Split catch-all modules when they combine
   unrelated concerns.
8. **Use methods when they clarify ownership.** Behavior that depends on a type's
   invariants lives with that type.
9. **Touched docs need timestamps.** Any doc changed in a patch updates its
   `Last modified` field. **Always run `date '+%Y-%m-%d %H:%M %Z'`**, never guess.
10. **Time transparency.** Operations longer than ~1 second must surface to all
    UI channels (console, TUI, desktop, dashboard) via the `progress_v2` event
    channel (`batchalign/worker/_protocol.py:write_progress_event`,
    `batchalign/worker/_progress.py`). Silent waits are UX bugs. Applies to model
    downloads, model loads, external API calls, any blocking wait. Full rationale:
    [`book/src/batchalign/architecture/time-transparency.md`](book/src/batchalign/architecture/time-transparency.md).

## Red/Green TDD: start at the top, drill down

Every feature and bug fix starts with a failing test, and the **first** failing
test is the highest-level integration test for the actual boundary the change
lives at. Unit tests on helpers are additional guards, never substitutes.

| Bug lives at... | Top-level test invokes... |
|-----------------|---------------------------|
| BA3 daemon dispatch | HTTP POST to local `batchalign3 daemon` / `batchalign3 benchmark` |
| Worker engine selection | `load_*_engine(bootstrap)` with `monkeypatch.setattr` on the model loader |
| Rust PyO3 boundary | round-trip a real `WorkerV2Request` JSON through `execute_*_request_v2` |
| CLI argument parsing | `subprocess.run(["batchalign3", ...])` |
| CHAT transform over the model | a real CHAT fragment through `batchalign_transform::...` (generic surface comes from chatter) |

Rationale: a past release shipped multiple show-stopper defects that every
unit test passed, because none of the tests exercised the real seams
(CLI subprocess, engine loading, end-to-end pipeline). Unit-only
TDD = false green.

## Critical policy: fix root causes, never symptoms

Trace a bug to its architectural origin and fix it there. No "pragmatic"
workarounds that mask the real problem. When a bug reveals a wrong architecture,
fix the architecture.

## Rust Coding Standards

Rust **2024 edition**. Follow the project's cross-repo coding charter
(operator-maintained). High-frequency points: typed errors over panics; no
silent swallowing (`.ok()`/`.unwrap_or_default()` that hides bugs); newtypes over
primitives at boundaries; enums (with `clap::ValueEnum`) over `--flag`/`--no-flag`
pairs; `BTreeMap` for deterministic JSON in tests; `LazyLock<Regex>` for constant
patterns; files <= ~400 lines (hard limit 800); no global mutable state, inject
dependencies for test control.

## Debugging Recipes

Canonical: `book/src/batchalign/developer/tracing-and-debugging.md`
(py-spy over workers) and `cpu-profiling.md` (tokio-console,
`debug-runtime` feature). Do not restate the recipes here.

## %mor / morphotag note

Batchalign emits Universal Dependencies (UD) `%mor` syntax (hyphen-separated
features, sentence-case tags), consumed/validated by chatter. Legacy CLAN-mor `&`
fusional markers are not produced. The canonical %mor/validation rules live in
chatter; this repo produces UD-tagged output and relies on chatter to validate it.
