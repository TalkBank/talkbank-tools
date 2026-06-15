# GitHub Readiness and Open Source Governance

**Status:** Current
**Last updated:** 2026-04-29 10:39 EDT

## Objective
Prepare `talkbank-tools` to operate as a healthy public project with clear legal, security,
contribution, and release processes.

## Root Artifacts

| Artifact | Status | Notes |
|----------|--------|-------|
| `LICENSE` | Done | BSD-3-Clause, `license.workspace = true` in all crates |
| `CONTRIBUTING.md` | Done | Setup, standards, PR flow, pre-PR checklist |
| `CODE_OF_CONDUCT.md` | Done | Root file added; adopts Contributor Covenant 2.1 with repo contact |
| `SECURITY.md` | Done | Root file added; issue-template contact link now resolves to a real policy |
| `CODEOWNERS` | **TODO** | Not added yet: repo contents do not currently publish an authoritative GitHub owner/team map for path-level review ownership |
| `.github/workflows/*.yml` | Done | `ci.yml` (core Rust, grammar, docs, fuzz, smoke, and summary jobs) + `release.yml` (multi-platform) |
| `.github/ISSUE_TEMPLATE/*` | Done | Bug report + feature request (YAML forms) |
| Pull request template | Done | `.github/PULL_REQUEST_TEMPLATE.md` mirrors current CONTRIBUTING + PR review requirements |

## CI Governance Policy

All items below are implemented in `ci.yml`:
- Required status checks: mirrored local gates where CI covers them (G0, G1, G2, G4–G10, G12), plus grammar, roundtrip/smoke jobs, docs integrity (`chat-anchors-check`), dependency audit, semver, fuzz smoke, and the aggregate summary job.
- `ci-report` summary job aggregates all required gates into a single merge check.
- Branch protection rules: documented in `book/src/contributing/branch-protection.md` — configure on GitHub once repo is public.

## Release Governance

- Pre-1.0 release cadence and tagging strategy: **TODO**.
- Changelog policy: **TODO** — to be defined at first public release.
- Release checklist: `release.yml` validates tag matches Cargo.toml version and builds multi-platform binaries. Documented checklist artifact: **TODO**.

## Community Operations

- Label taxonomy: `bug` and `enhancement` auto-applied by issue templates. Richer taxonomy (`drift`, `spec`, `grammar`, `parser`, `docs`, `good first issue`): **TODO** (GitHub settings).
- Contributor pathway: `CONTRIBUTING.md` covers setup and PR flow. First-time/advanced contributor pathways: **TODO**.
- Public project roadmap: **TODO**.

## Supply Chain and Security

- Dependency scanning: CI runs `rustsec/audit-check` and `cargo-deny` (with `deny.toml`). Automated update PRs (Dependabot/Renovate): **TODO**.
- Signed release artifacts: **TODO**.
- Security advisories process: documented in `SECURITY.md`.

## Acceptance Criteria
- Repo has complete governance artifacts at root.
- CI and branch protections enforce stated policy.
- Contributors can onboard and submit PRs without tribal knowledge.
- Release process is repeatable and documented.
