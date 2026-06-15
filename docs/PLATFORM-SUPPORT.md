# Platform Support Matrix

**Status:** Current
**Last updated:** 2026-04-29 10:26 EDT

This document answers two separate questions for each public-facing surface in
`talkbank-tools`:

1. **Is this a supported public surface?** See also
   [`RELEASE-CONTRACT.md`](RELEASE-CONTRACT.md).
2. **What platform claim is accurate today?** That includes CI coverage,
   release artifacts, and important caveats.

## Quick surface summary

| Surface | Release/support status | Accurate platform claim today |
|---|---|---|
| `chatter` CLI | Public stable | Release binaries for Linux x86_64, macOS arm64/x86_64, and Windows x86_64 |
| Public Rust core crates (`talkbank-model`, `talkbank-parser`, `talkbank-transform`, `talkbank-clan`) | Public preview | Source-first cross-platform surface; Linux is the only full PR test gate today |
| `tree-sitter-talkbank` grammar | Public preview | Reusable cross-platform grammar package; grammar CI currently runs on Ubuntu |
| `batchalign3` CLI / local server / dashboard | Public preview | Wheels for macOS arm64/x86_64, Linux x86_64/aarch64, and Windows x86_64 |
| Batchalign Desktop (`apps/dashboard-desktop/`) | Experimental | In-repo Batchalign GUI shell only; no supported release distribution |

## Support level definitions

- **Tier A:** Full PR CI gate for that surface on the named platform.
- **Tier B:** Public release artifact or explicit smoke coverage exists, but not a
  full PR test gate.
- **Tier C:** Experimental/in-repo only. No supported end-user guarantee.

## `chatter` CLI

| Platform | Tier | CI / artifact evidence | Notes |
|---|---|---|---|
| Linux x86_64 | A | PR CI runs tests and `chatter` smoke; release archives built | Strongest current support story |
| macOS arm64 | B | Cross-platform smoke in PR CI; release archive built | Native smoke coverage exists, but not a full test matrix |
| macOS x86_64 | B | Release archive built | Build coverage only |
| Windows x86_64 | B | Cross-platform smoke in PR CI; release archive built | Smoke coverage exists, but not a full test matrix |

## Public Rust core crates

These are the documented source-first Rust dependency surfaces today:
`talkbank-model`, `talkbank-parser`, `talkbank-transform`, and `talkbank-clan`.

| Platform | Tier | CI / artifact evidence | Notes |
|---|---|---|---|
| Linux x86_64 | A | PR CI runs crate tests directly | Canonical source-level gate |
| macOS arm64 | B | Compiled transitively in core release builds | No crate-level macOS test job yet |
| macOS x86_64 | B | Compiled transitively in core release builds | No crate-level Intel macOS test job yet |
| Windows x86_64 | B | Compiled transitively in core release builds | No crate-level Windows test job yet |

These crates are supported as **source** surfaces, not as separate prebuilt
downloads or published crates.io installs today.

## `tree-sitter-talkbank` grammar

| Aspect | Tier | Current claim |
|---|---|---|
| Grammar source + generated artifacts | A on Linux CI, B elsewhere | Grammar generation and `tree-sitter test` run in Ubuntu CI |
| Standalone grammar packages | B | Public preview reusable package line (`npm`, `crates.io`, and PyPI metadata) |
| Downstream bindings | B for Rust, C for others | Rust is the primary maintained binding; other bindings in `bindings/` are still scaffold-level |

Use cross-platform language here carefully: the grammar itself is intended to be
portable, but the CI proof today is Ubuntu-only and the binding story is not
uniform across languages.

## `batchalign3` CLI / local server / dashboard

| Platform | Tier | CI / artifact evidence | Notes |
|---|---|---|---|
| Linux x86_64 | A | Wheel smoke + PR CI on Ubuntu | Primary Batchalign CI platform |
| Linux aarch64 | B | Release wheel built | Build coverage only |
| macOS arm64 | B | Wheel smoke + release wheel | Stronger than build-only, but not a full PR suite |
| macOS x86_64 | B | Release wheel built | Build coverage only |
| Windows x86_64 | B | Wheel smoke + release wheel | CLI install/help path is covered; server/worker mode still has limitations |
| Dashboard UI (browser) | B | Built in CI; desktop/web frontend shared | Supported as part of the preview Batchalign surface, but the API is not frozen for third parties |

Important Windows caveat: server/worker lifecycle code still has Unix-specific
gaps, so avoid describing Windows as feature-identical to Linux/macOS for every
Batchalign deployment mode.

## Desktop surfaces

The repo contains one experimental desktop app:

- **Batchalign Desktop** = `apps/dashboard-desktop/` = experimental native shell
  around the Batchalign processing/dashboard UI.

### Batchalign Desktop (`apps/dashboard-desktop/`)

| Platform | Tier | Current claim |
|---|---|---|
| macOS / Windows | C | Experimental artifact workflow exists, but this is not a supported end-user release line |
| Linux | C | No supported public desktop distribution |

## How to phrase support publicly

- Say **"`chatter` is the stable public CHAT-first CLI"**.
- Say **"`batchalign3` is the public preview audio/ML surface"**.
- Say **"the Rust crates are public preview and currently source-first via
  git/path dependencies"**.
- Say **"`tree-sitter-talkbank` is public preview"**, not "fully stable across
  all bindings".
- Say **"Batchalign Desktop"** by name.
- Do **not** describe the desktop app as a generally supported end-user
  release surface today.
