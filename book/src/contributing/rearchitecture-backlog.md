# Rearchitecture Backlog

**Status:** Historical
**Last updated:** 2026-03-23 23:49 EDT

This page records future work surfaced during architecture review, code
cleanup, and refactors. It is intentionally narrower than a general TODO list:
items here should describe real structural follow-up work.

## High-Value Opportunities

### 1. Reduce overlap/alignment surface area

The new overlap grouping and walker model is a better foundation, but there are
still many entrypoints and helper layers involved in alignment behavior.

Potential directions:

- consolidate overlap/alignment planning into fewer public model-layer APIs
- keep audit/debug commands as thin wrappers over those APIs
- remove remaining adapter-local knowledge from CLI and LSP callers

### 2. Make spec reconciliation routine

This review removed a significant amount of dead or stale error-spec material.
That is useful cleanup, but it should not require another large sweep.

Potential directions:

- add or strengthen checks that compare live codes, generated schema, and spec
  inventory
- make dead-spec deletion part of normal review hygiene
- keep "implemented" status close to the code that proves it

### 3. Push domain typing earlier in header parsing

The header-language cleanup improved the model, but some parsing and adapter
paths still spend too long in raw-string form before converting.

Potential directions:

- convert to typed language/header values at parse boundaries
- avoid lossy or delayed conversions in participant/header helpers
- keep reporting/rendering layers separate from storage/validation types

### 4. Clarify test regeneration ownership

Snapshot-heavy repos need explicit regeneration policy or reviewers end up
treating large snapshot deltas as unavoidable noise.

Potential directions:

- document which suites are expected to regenerate during structural refactors
- add smaller focused fixtures for high-risk overlap/header cases
- keep broad snapshot suites, but give targeted suites a lower-noise path

### 5. Consolidate repo-local tooling under typed Rust entrypoints

The repo has accumulated useful checks, generators, and guardrails, but the
entrypoints are still spread across Make, shell, Python, Node, and Rust bins.
That encourages string-based orchestration drift.

Potential directions:

- keep `cargo xtask` as the canonical home for repo-local orchestration
- keep shell wrappers thin and ecosystem-specific instead of letting them own
  dependency-aware planning logic
- reserve standalone Rust bins for real domain tooling (`spec/tools`,
  generators, analyzers), not ad hoc local glue

### 6. Keep the core model free of binding-only concerns

The `talkbank-model` crate no longer owns PyO3 extraction glue, and channel-
backed error sinks are now opt-in rather than part of the default core-model
surface. That is the right direction, but more cleanup remains.

Potential directions:

- keep Python extraction wrappers at the actual binding edge instead of in the
  shared model crate
- keep optional transport/runtime helpers behind explicit features rather than
  making them default model dependencies
- reconsider whether the current `Provenance<M, T>` name matches its real role
  as semantic boundary tagging rather than true runtime provenance

### 7. ~~Retire legacy synthetic tree-sitter fragment APIs~~ (Completed/Obsolete)

The direct parser (Chumsky) has been removed. Tree-sitter is the sole parser.
The synthetic fragment APIs and direct-parser bootstrap infrastructure that
motivated this item no longer exist. No further action needed.

### 8. ~~Rebuild direct-parser testing around direct semantics~~ (Completed/Obsolete)

The direct parser (Chumsky) has been removed. Tree-sitter is the sole parser.
All direct-parser-specific testing concerns (fragment semantics, recovery
suites, parity gates) are obsolete. No further action needed.

### 9. ~~Replace the bootstrap-era spec/generation architecture~~ (Completed/Obsolete)

The direct parser (Chumsky) has been removed. The bootstrap-era dual-parser
architecture that created circular dependencies and generation sprawl no longer
exists. The spec/generation system now targets tree-sitter only. The split of
`spec/runtime-tools` from `spec/tools` was completed. No further action needed.
