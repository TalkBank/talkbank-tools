# CI and Release

**Status:** Current
**Last updated:** 2026-05-21 13:05 EDT

## Pre-Merge Verification

Every change must pass `make verify` before merging. This runs gates G0 through G14:

```bash
make verify
```

See [Testing > Verification Gates](testing.md#verification-gates) for the full gate table.

## Generated Artifact Check (G12)

`make generated-check` regenerates all artifacts and verifies they match what's committed:

- Symbol sets (Rust and JavaScript)
- Tree-sitter corpus tests
- Rust parser tests
- Error documentation

If the check fails, it means committed generated artifacts are out of sync with
their source inputs. That usually means specs or symbols changed without the
corresponding regeneration step, not that every parser change should have run
`make test-gen`.

## Parser Signature Guardrail (G0)

`make parser-guard` enforces a coding convention: parser functions should use consistent `ErrorSink` signatures. This prevents accidental introduction of incompatible parser APIs.

## Release Process

Releases are currently manual and split by surface. Use the workflow that
matches the release contract for the surface you are shipping.

| Workflow | Scope | Tier | Output | Guardrails |
|----------|-------|------|--------|------------|
| `.github/workflows/release.yml` (`TalkBank Core Release`) | Canonical Rust binary release line for `chatter` | Public stable | GitHub Release assets for the shared Rust workspace version | Requires an existing `vX.Y.Z` tag matching `Cargo.toml`; runs native binary smoke tests before publishing; does **not** cargo-publish the Rust library crates |
| `.github/workflows/batchalign-release.yml` (`Batchalign Package Release (Preview)`) | Public `batchalign3` package/CLI release line | Public preview | Wheels + sdist, with optional GitHub Release and optional PyPI publish | Requires an existing `vX.Y.Z` tag matching `pyproject.toml`; runs wheel smoke tests before publish |
| `.github/workflows/batchalign-desktop.yml` (`Batchalign Desktop Bundles (Experimental)`) | Dashboard desktop shell bundle builds | Experimental | GitHub Actions artifacts only | Artifact-only internal validation; does not create a public GitHub release |

General process:

1. Run the relevant verification for the surface you are releasing (`make verify`
   for the core Rust release line, and the Batchalign packaging checks for
   `batchalign3`).
2. Update the canonical version source for that surface.
3. For the core Rust and Batchalign package release lines, create and push the
   matching `vX.Y.Z` git tag before dispatching the workflow.
4. Trigger the surface-specific workflow.
5. Only promote a preview or experimental surface by updating
   `docs/RELEASE-CONTRACT.md`, `docs/VERSIONING.md`, and the corresponding
   workflow labeling together.

## First-release release-ops guardrails

- **Batchalign release smoke must cover `batchalign3 serve` + `/health`.** The
  wheel smoke path is not complete if it only checks `--help` and `version`.
- **Signing/notarization language must follow `docs/code-signing-and-distribution.md`.**
  Current GitHub Release archives and wheels are not a license to
  imply native-installer trust guarantees we do not yet automate.

## Cross-Surface Testing

Batchalign now lives inside this workspace as the `batchalign-*` sibling
crates plus the Python package at `batchalign/`. After changes to core
Rust crates that the Python extension depends on, rebuild the bridge
and run the Python suite from the workspace root:

```bash
uv run maturin develop    # Rebuild the batchalign-pyo3 extension in-place
uv run pytest batchalign  # Python test suite
```

CLI and CLAN tests run as part of the main workspace's
`make test` and `make verify`.
