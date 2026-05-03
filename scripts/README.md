# scripts/

**Status:** Current
**Last updated:** 2026-04-28 23:01 EDT

This directory is a shared toolbox for repo maintenance, generated-artifact
refreshes, drift checks, targeted smoke tests, diagnostics, and fixture prep.
For normal contributor work, start with repo-native entrypoints first:

- `make help` for supported top-level tasks
- `cargo run -q -p xtask -- help` for lower-level Rust maintenance helpers

Most scripts here are wrappers or focused helpers behind those entrypoints. Use
the index below when you need the direct script.

## Start here

| I want to... | Prefer this entrypoint | Notes |
|---|---|---|
| Run the normal contributor gates | `make check`, `make test`, `make verify`, `make ci-local`, `make ci-full` | Prefer these over ad hoc script chains. |
| Install the repo’s local git guardrail | `make install-hooks` | Installs `scripts/pre-push.sh`. |
| Regenerate spec-driven or grammar-driven artifacts | `make symbols-gen`, `make test-gen`, `scripts/update-tree-sitter.sh` | Use the Make targets unless you are doing grammar-specific regeneration. |
| Verify generated artifacts or guardrails are still in sync | `make generated-check`, `make parser-guard`, `make chat-anchors-check`, `make check-specs` | These are the canonical drift/guardrail entrypoints. |
| Verify imported Batchalign generated surfaces | `make batchalign-dashboard-api-check`, `make batchalign-runtime-check`, `bash scripts/check_ipc_type_drift.sh` | Use the first two via `make`; IPC schema drift is still a direct script. |
| Run a focused dashboard smoke or E2E helper | `bash scripts/run_react_dashboard_smoke.sh` | Builds the frontend dependencies it needs. |
| Prepare a minimal repro fixture from real CHAT + media | `python3 scripts/trim_chat_audio.py ...` or `python3 scripts/prepare_corpus_media_fixture.py ...` | Prefer these over hand-editing fixtures. |

## Taxonomy

### Generators and regeneration

| Script | What it does | Canonical entrypoint |
|---|---|---|
| `generate-symbol-sets.js` | Regenerates shared symbol sets used across the grammar and Rust crates. | `make symbols-gen` |
| `generate-node-types.js` | Regenerates Rust node-type constants from tree-sitter output. | `scripts/update-tree-sitter.sh` |
| `generate_dashboard_api_types.sh` | Regenerates dashboard API artifacts from the Batchalign server surface. | Called by `make batchalign-dashboard-api-check` and dashboard smoke scripts |
| `generate_ipc_types.sh` | Regenerates Python IPC models from Rust JSON Schema. | Direct script after Rust↔Python boundary changes |
| `generate_stanza_language_table.py` | Refreshes the hardcoded Stanza support table from upstream resource data. | Direct script after Stanza/resource updates |
| `update-tree-sitter.sh` | Regenerates tree-sitter parser/bindings and reruns grammar tests. | Direct script after `grammar.js` changes |

### Drift checks and guardrails

| Script | What it checks | Canonical entrypoint |
|---|---|---|
| `check-errorsink-option-signatures.sh` | Parser guardrail for `ErrorSink` + `Option` signatures. | `make parser-guard` |
| `check-chat-manual-anchors.sh` | Verifies CHAT manual anchors referenced in docs/source still resolve. | `make chat-anchors-check` |
| `check-error-specs.sh` | Ensures every error-code enum entry has a matching spec file. | `make check-specs` |
| `check_dashboard_api_drift.sh` | Verifies generated dashboard API artifacts are up to date. | `make batchalign-dashboard-api-check` |
| `check_ipc_type_drift.sh` | Verifies IPC JSON Schema artifacts match committed sources. | Direct script after IPC type changes |
| `check_runtime_drift.py` | Verifies shared runtime constants still parse and expose expected keys. | `make batchalign-runtime-check` |
| `run-drift-probes.sh` + `generate_drift_report.py` | Runs drift probes and renders a report for inspection. | Direct scripts for targeted investigation |

### Smoke tests and targeted diagnostics

| Script | What it helps with | Preferred use |
|---|---|---|
| `pre-push.sh` | Fast local gate that mirrors key CI checks. | Install via `make install-hooks` |
| `run_react_dashboard_smoke.sh` | Frontend/dashboard smoke and E2E flow. | Direct script |
| `build_react_dashboard.sh` | Builds the dashboard bundle into the configured target directory. | Direct script |
| `test-single-file.sh` | Quick roundtrip test for one CHAT file. | Direct script |
| `test-bg.sh` + `test-bg-status.sh` | Fire-and-forget background test runner plus status view. | Direct scripts for long local runs |
| `test-affected.py` | Computes a test subset from changed files and `affects:` annotations. | Direct script for targeted local testing |
| `choose-test-concurrency.sh` | Picks a safer test parallelism based on memory. | Support helper for local test workflows |
| `test_lazy_profile_e2e.sh` | Focused E2E check for LazyProfile worker behavior. | Direct script |
| `temporal-stress-test.sh` | Stress harness for the Temporal workflow path. | Specialized; currently marked dormant in-script |
| `fetch-metrics-trend.sh` + `metrics-snapshot.sh` | Pulls or emits metrics snapshots for quality tracking. | Direct scripts for metrics/debugging |
| `compare_check_parity.py` | Compares CLAN CHECK output against `chatter clan check`. | Direct diagnostic when parity drifts |
| `divergence_diagnosis.py` | Pinpoints JSON divergence between parser implementations. | Direct diagnostic after a failing divergence capture |
| `audit_continuations.py` | Audits grammar regexes around continuation handling. | Direct diagnostic for grammar work |
| `audit_documentation.py` | Audits model documentation coverage. | Direct diagnostic for docs/code sync |

### Fixture prep and corpus maintenance

| Script | What it does | Preferred use |
|---|---|---|
| `trim_chat_audio.py` | Trims a CHAT file and matching audio to a focused utterance range. | Minimal repro fixture creation |
| `prepare_corpus_media_fixture.py` | Copies a CHAT file plus matching media, then delegates trimming. | Fixture prep when media lives on a remote host |
| `generate_check_error_corpus.py` | Generates minimal files intended to trigger specific CHECK errors. | Direct script for CHECK-fixture generation |
| `synthesize_check_corpus.py` | Synthesizes and verifies a CHECK error corpus against CLAN. | Direct script for parity-fixture work |
| `capture_check_golden.sh` | Captures CLAN CHECK golden output for the error corpus. | Direct script when refreshing goldens |
| `fix_corpus_ages.pl` | Normalizes `@ID` ages in `corpus/reference/`. | Direct maintenance script |
| `fix_test_fixture_ages.pl` | Normalizes `@ID` ages in tests/spec fixtures outside the reference corpus. | Direct maintenance script |

### Analysis support

`scripts/analysis/` currently holds supporting inputs and baselines for focused
audits, not a general contributor entrypoint. Treat those files as analysis
artifacts unless a script explicitly points at them.

## Rules of thumb

- Prefer the `make` or `xtask` entrypoint when one exists; it captures the
  supported workflow and usually composes multiple lower-level steps.
- Reach for direct scripts when you need a focused generator, a narrow
  diagnostic, or fixture-prep helper that does not have a stable top-level
  target.
- If you add a new script that contributors might reasonably discover by
  browsing, add it to this index in the matching category.
