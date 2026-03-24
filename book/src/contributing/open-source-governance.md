# GitHub Readiness and Open Source Governance

**Status:** Current
**Last updated:** 2026-03-24 00:01 EDT

## Objective
Prepare `talkbank-tools` to operate as a healthy public project with clear legal, security,
contribution, and release processes.

## Root Artifacts

| Artifact | Status | Notes |
|----------|--------|-------|
| `LICENSE` | Done | BSD-3-Clause, `license.workspace = true` in all crates |
| `CONTRIBUTING.md` | Done | Setup, standards, PR flow, pre-PR checklist |
| `CODE_OF_CONDUCT.md` | **TODO** | |
| `SECURITY.md` | **TODO** | Issue template config links to it but file doesn't exist yet |
| `CODEOWNERS` | **TODO** | Path-level ownership |
| `.github/workflows/*.yml` | Done | `ci.yml` (11 jobs, G0–G10 gates) + `release.yml` (multi-platform) |
| `.github/ISSUE_TEMPLATE/*` | Done | Bug report + feature request (YAML forms) |
| Pull request template | **TODO** | |

## CI Governance Policy

All items below are implemented in `ci.yml`:
- Required status checks: compile, test gates (G0–G10), generation drift, lint/format, docs integrity (`chat-anchors-check`), dependency audit (RustSec + cargo-deny), semver check.
- `ci-report` summary job aggregates all required gates into a single merge check.
- Branch protection rules: documented in `book/src/contributing/branch-protection.md` — configure on GitHub once repo is public.

## Release Governance

- Pre-1.0 release cadence and tagging strategy: **TODO**.
- Changelog policy: per-crate `CHANGELOG.md` files exist (7 crates). Root-level changelog and labeling policy (breaking vs non-breaking): **TODO**.
- Release checklist: `release.yml` validates tag matches Cargo.toml version and builds multi-platform binaries. Documented checklist artifact: **TODO**.

## Community Operations

- Label taxonomy: `bug` and `enhancement` auto-applied by issue templates. Richer taxonomy (`drift`, `spec`, `grammar`, `parser`, `docs`, `good first issue`): **TODO** (GitHub settings).
- Contributor pathway: `CONTRIBUTING.md` covers setup and PR flow. First-time/advanced contributor pathways: **TODO**.
- Public project roadmap: **TODO**.

## Supply Chain and Security

- Dependency scanning: CI runs `rustsec/audit-check` and `cargo-deny` (with `deny.toml`). Automated update PRs (Dependabot/Renovate): **TODO**.
- Signed release artifacts: **TODO**.
- Security advisories process: **TODO** (blocked on `SECURITY.md`).

## Acceptance Criteria
- Repo has complete governance artifacts at root.
- CI and branch protections enforce stated policy.
- Contributors can onboard and submit PRs without tribal knowledge.
- Release process is repeatable and documented.
