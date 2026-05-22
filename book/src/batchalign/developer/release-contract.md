# Release Contract

**Status:** Current
**Last updated:** 2026-05-20 00:47 EDT

This document defines the stability tiers for all public surfaces of the
batchalign3 project. Consumers can use these tiers to decide which surfaces
are safe to depend on and which may change without notice.

## Release state

**Beta (1.0.0-beta).** Public release pending stabilization of packaging,
cross-repo dependencies, and test strategy.

## Stable surfaces (target for 1.0)

These surfaces will have committed APIs at 1.0. Breaking changes will follow
semver after that point.

- **CLI (`batchalign3`)** -- transcribe, align, morphotag, utseg, translate,
  coref, opensmile, avqi.
- **Python package (`batchalign3` on PyPI)** -- CLI entry point plus the
  `batchalign_core` Rust extension module.
- **Local server mode (`batchalign3 serve`)** -- single-machine job execution
  with REST + WebSocket interface.

## Preview surfaces

These surfaces are functional and used in production, but their APIs may change
between minor versions.

- **Dashboard (React web UI)** -- functional, under active development.
- **REST API** -- used by the dashboard. Schema is documented but not frozen.

## Experimental / dormant surfaces

These surfaces exist in the repo but are not packaged, not tested in CI, and
not covered by any compatibility promise.

- **Desktop app (Tauri)** -- dormant, not functional.
- **Installer scripts** -- partially implemented.
- **Staged / remote execution** -- experimental.

## Workspace dependency

The batchalign crates live as siblings inside the talkbank-tools
Cargo workspace (`batchalign3` was folded into talkbank-tools on
2026-04-28). The runtime crate `batchalign` depends on the
talkbank-* sibling crates by workspace path:

- `talkbank-model` — CHAT data model + validation
- `talkbank-parser` — tree-sitter-backed CHAT parser
- `talkbank-transform` — pipelines, alignment, CHAT↔JSON

`talkbank-clan` is NOT a runtime dependency of the batchalign crate;
it is a sibling crate used by the standalone `chatter` CLI.

Before 1.0 the talkbank-* sibling crates target being published to
crates.io with stable versioned APIs; the batchalign runtime will
follow that publish cadence. Until then, the workspace path
dependencies are the single source of truth and there is no
separate cross-repo release boundary.

## Platform support

| Tier | Platforms | Meaning |
|------|-----------|---------|
| **A** (CI-tested) | Linux x86_64 | Every PR runs tests on this target |
| **B** (release builds) | macOS ARM, macOS Intel, Linux ARM, Windows x86_64 | Release binaries are built but not exercised by CI |

**Note:** Process lifecycle code uses Unix-specific APIs (signals, process
groups). Windows support is build-only -- the server and worker subsystems are
not expected to function on Windows without porting work.

## License

BSD-3-Clause.
