# Rearchitecture Backlog

**Status:** Current
**Last updated:** 2026-03-21

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

### 7. Retire legacy synthetic tree-sitter fragment APIs

The public tree-sitter fragment helpers in `talkbank-parser` were useful while
the direct parser was being bootstrapped, but they now hide synthetic-file
parsing behind names that sound like true fragment parsers.

Potential directions:

- done in this tranche: make `spec/runtime-tools` the home for
  bootstrap/mining/runtime-validation tooling instead of keeping those paths in
  `spec/tools`
- done in this tranche: switch the pre-merge fragment gate to
  direct-parser-native recovery tests and demote tree-sitter word-fragment
  parity to a legacy audit target
- done in this tranche: move runtime word-description generation and the
  component roundtrip tests for words/utterances onto `DirectParser`
- done in this tranche: remove `parse_word()`, `parse_main_tier()`, and
  `parse_utterance()` from the `talkbank_parser` crate root and force the
  remaining synthetic helpers under the explicit
  `talkbank_parser::synthetic_fragments` namespace
- stop using fake tree-sitter fragment behavior as the oracle for direct-parser
  fragment tests
- keep any remaining synthetic wrappers clearly internal and explicitly named
  as synthetic/test-only helpers

### 8. Rebuild direct-parser testing around direct semantics

The direct parser is no longer just a strict fragment parser. It has selective
lenient/recovery behavior, which means the old bootstrap-era strategy of
"compare fragments to tree-sitter" is not strong enough.

Potential directions:

- done in this tranche: establish `make test-fragment-semantics` as the real
  fragment-semantic gate and `make test-legacy-fragment-parity` as an explicit
  migration audit
- split full-file equivalence from fragment-semantic testing instead of mixing
  them under one notion of "parser parity"
- replace `golden_unit_tests.rs` as the primary fragment oracle with spec-led
  fixtures and direct invariants
- add focused recovery suites for dropped tiers, retained valid siblings,
  parse-health taint propagation, and diagnostic monotonicity
- add property tests and mutation/fuzz-style checks for idempotence,
  serialization stability, and "leniency without silent fabrication"

### 9. Replace the bootstrap-era spec/generation architecture

The current spec and generation system still assumes too much of the old direct-
parser bootstrap story. That creates unnecessary circular dependencies,
generation sprawl, and misleading authority boundaries.

Potential directions:

- done in this tranche: split `spec/runtime-tools` out of `spec/tools` so the
  core generator crate no longer owns bootstrap-era parser/model coupling
- keep fragment specs, but stop treating synthetic tree-sitter wrapper behavior
  as fragment-semantic truth
- separate grammar corpus generation, direct-parser semantic tests, full-file
  parity tests, and validation/error specs into distinct tracks
- narrow `spec/tools` to true artifact generation and spec validation instead
  of letting it own bootstrap-era parser/model coupling
- replace giant regeneration rituals with smaller affected workflows that match
  the actual type of change
