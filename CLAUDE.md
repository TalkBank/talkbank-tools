# CLAUDE.md

**Last modified:** 2026-07-01 09:19 EDT

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
chatter on 2026-06-18 (meta-repo memory
`feedback_atomic_repo_partition_migrations`).

### How the CHAT core is consumed

`[workspace.dependencies]` in `Cargo.toml` points `talkbank-model`,
`talkbank-parser`, `talkbank-parser-re2c`, `talkbank-parser-tests`, and
`talkbank-transform` at the published, public chatter via git deps pinned to a
release tag: `{ git = "https://github.com/TalkBank/chatter", tag = "v0.2.1" }`.
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
| `batchalign-types` | Shared types |
| `send2clan-sys` | CLAN-app FFI; currently **orphaned** (no consumer) , candidate for removal |

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

```bash
make verify              # pre-merge gate: workspace compile + batchalign check/tests + mdBook
make batchalign-ci-rust  # batchalign Rust gate (check + lib tests + integration + pyo3 build)
cargo check --workspace --all-targets
cargo nextest run -p batchalign --lib
cargo nextest run -p batchalign-transform
cargo fmt
cargo clippy --all-targets -- -D warnings
bash scripts/lint/shellcheck-all.sh   # every tracked shell script, strictest severity
```

`make verify` and the pre-push hook verify the **batchalign layer only**;
CHAT-format verification (parser-equivalence, generated artifacts, fuzz, corpus
roundtrip, grammar) is chatter's job now. CI workflows: `ci.yml` (cross-cutting
dependency-audit + shellcheck), `batchalign-rust.yml`, `batchalign-python.yml`,
`batchalign-desktop.yml`, `book.yml`.

## Releases and Versioning

`batchalign3 version` reports `batchalign3 <pkg> (build <pkg>-<git-describe>-<epoch>)`,
assembled in `crates/batchalign/build.rs` as `BUILD_HASH` (`git describe --always
--dirty`). The epoch changes on every rebuild, so a stale binary is detectable in
development. With no reachable tag the describe is a bare commit; once an annotated
`vX.Y.Z` tag is reachable it reads `vX.Y.Z-<n>-g<commit>`. No release semver is baked
into the binary; the build hash is the identity. When matching the commit, key on git's
`-g<hex>` describe sentinel (bare-commit fallback); never split the string by `-`
position.

Releases are GitHub Releases, **not PyPI**. `.github/workflows/batchalign-release.yml`
is triggered by a `v*` tag push (or `workflow_dispatch`, which offers a `dry_run` that
builds and smoke-tests without publishing); it attaches the installer scripts, abi3
wheels, and a checksum file. End users install from the GitHub Release via a uv-bootstrap
installer pulling those wheels, never from a package index.

The CHAT core is consumed from chatter at a pinned **release tag** (the `tag = "v0.2.1"`
git dep shown above); adopt a newer chatter by bumping that tag and running `cargo update`.

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

Rationale: the 2026-05-26 Cantonese ASR ship had three show-stoppers (schema
rejected `qwen_model` overrides; benchmark discarded runs on one bad token; `yue`
defaulted to the worst engine) that every unit test passed because none exercised
the actual seams. Unit-only TDD = false green.

## Critical policy: fix root causes, never symptoms

Trace a bug to its architectural origin and fix it there. No "pragmatic"
workarounds that mask the real problem. When a bug reveals a wrong architecture,
fix the architecture.

## Rust Coding Standards

Rust **2024 edition**. Follow the cross-repo charter in the meta-repo
`docs/coding-standards.md`. High-frequency points: typed errors over panics; no
silent swallowing (`.ok()`/`.unwrap_or_default()` that hides bugs); newtypes over
primitives at boundaries; enums (with `clap::ValueEnum`) over `--flag`/`--no-flag`
pairs; `BTreeMap` for deterministic JSON in tests; `LazyLock<Regex>` for constant
patterns; files <= ~400 lines (hard limit 800); no global mutable state, inject
dependencies for test control.

## Debugging Recipes (Python workers, async runtime)

**py-spy , Python CPU profiler / hung-worker triage.** Reads a running Python
process by PID without restarting it. First thing to try when a worker is hung at
0% CPU. `brew install py-spy` or `uv pip install py-spy`.

```bash
sudo py-spy dump --pid <worker-pid>                      # one-shot stack dump
sudo py-spy top  --pid <worker-pid>                      # live per-function CPU
sudo py-spy record -o profile.svg --pid $(pgrep -f batchalign3) --native --subprocesses
```

`--native` sees PyTorch / Stanza / Whisper internals; `--subprocesses` follows
forked children. `sudo` is required on macOS to read another process's memory.

**tokio-console , async runtime debugger for the Rust side.** Live TUI of every
async task, what it waits on and for how long. Use when Rust dispatch is parked on
a `oneshot::Receiver` / `Mutex` / `Semaphore`. Build behind the `debug-runtime`
feature (zero production impact); needs `--cfg tokio_unstable`.

```bash
RUSTFLAGS="--cfg tokio_unstable" cargo build -p batchalign --bin batchalign3 --features debug-runtime
./target/debug/batchalign3 transcribe input/ -o out/   # spawns gRPC server on 127.0.0.1:6669
tokio-console http://127.0.0.1:6669                     # in another terminal
```

Full guides: `book/src/batchalign/developer/tracing-and-debugging.md` and
`cpu-profiling.md`.

## %mor / morphotag note

Batchalign emits Universal Dependencies (UD) `%mor` syntax (hyphen-separated
features, sentence-case tags), consumed/validated by chatter. Legacy CLAN-mor `&`
fusional markers are not produced. The canonical %mor/validation rules live in
chatter; this repo produces UD-tagged output and relies on chatter to validate it.
