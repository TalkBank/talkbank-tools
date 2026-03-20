# Rearchitecture Backlog

**Status:** Current
**Last updated:** 2026-03-20

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
