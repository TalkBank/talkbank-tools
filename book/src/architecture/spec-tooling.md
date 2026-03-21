# Spec Tooling and Generation Pipeline

## Objective
Make `spec/` the reliable language-contract source while keeping generation
deterministic, maintainable, and appropriately scoped.

Post-bootstrap, the goal is no longer “one generation pipeline owns all parser
truth.” The goal is to separate:

- grammar artifact generation
- validation/error-doc generation
- direct-parser semantic testing
- full-file parity testing

Anything that still looks like bootstrap-era parser mining or synthetic
fragment orchestration is now audit-only unless a doc says it remains
operational.

## Current Risk Snapshot
- `spec/tools` currently fails compile in baseline (`CA_ANNOTATION` vs `ALT_ANNOTATION` drift).
- This demonstrates a missing hard contract between node type generation and tool consumption.
- `spec/tools` still carries bootstrap-era Rust parser/model dependencies that
  create circular or awkward workflow coupling.
- contributor workflows still over-assume that `make test-gen` is the right
  reaction to every parser-related change.

## Current Generation Pipeline

```text
spec constructs/errors
  -> spec validators
  -> generated grammar corpus tests
  -> generated rust parser/validation tests
  -> generated error docs
  -> coverage dashboards and quality reports
```

That pipeline is still useful, but it is too broad to remain the single mental
model for parser testing.

## Desired Post-Bootstrap Split

```text
grammar specs/templates
  -> generated tree-sitter corpus tests

error specs
  -> generated validation/parser error tests
  -> generated error docs

direct-parser semantic fixtures and invariants
  -> direct-parser-native tests

reference corpus / curated full files
  -> parser parity tests
```

## Structural Reorganization for `spec/tools`
- Split into explicit internal modules:
  - `input` (markdown/spec parsing)
  - `ir` (normalized internal representation)
  - `emit` (grammar tests, rust tests, docs)
  - `validate` (schema and semantic checks)
  - `sync` (grammar node-types and symbol-registry checks)
- Narrow its mission back to spec-driven artifact generation and validation,
  rather than letting it remain a bootstrap-era staging ground for parser
  semantics.

## Legacy vs Active

Keep these active:

- grammar corpus generation
- error doc generation
- symbol registry sync/validation
- affected regeneration when a spec or grammar input truly changed

Treat these as legacy audit paths:

- synthetic tree-sitter fragment wrappers
- bootstrap-era parser equivalence rituals
- any generation flow whose main purpose is to compare direct-parser fragments
  to a tree-sitter fragment oracle

## Determinism Requirements
1. Stable ordering of generated outputs.
2. Stable formatting of generated code/docs.
3. Re-runs without source changes produce no diffs.

## Drift Prevention Controls
- Node type compatibility check:
  - `spec/tools` must compile and run against current generated node constants.
- Registry compatibility check:
  - all symbol categories used in specs and grammar must be known in registry.
- Generation integration check:
  - full generation pass with clean tree must produce zero diff.
- Boundary check:
  - generated grammar/docs flows should not silently become the authority for
    direct-parser fragment semantics.

## Authoring Experience
- Provide strict but simple spec templates for constructs and errors.
- Add `spec lint` command for immediate feedback:
  - missing fields,
  - invalid tags,
  - malformed examples,
  - unknown error codes.
- Document when `make test-gen` is actually needed and when a small direct test
  is the right answer instead.

## Versioning and Metadata
Each spec file should include:
- ownership,
- status (`draft`, `accepted`, `deprecated`),
- parser/validation scope,
- linked tests and generated outputs.

## Acceptance Criteria
- `spec/tools` is green and deterministic.
- Every generation target has explicit provenance from source specs.
- Drift between node types, specs, and generators is blocked in CI.
- Spec contributors have a documented and automated happy path.
- Small grammar changes no longer force a giant regeneration ritual by default.
- Direct-parser fragment semantics are tested outside the generation pipeline.
