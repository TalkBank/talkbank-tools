# Spec Tooling and Generation Pipeline

## Objective
Make `spec/` the reliable language contract source while making generators deterministic,
maintainable, and impossible to drift from grammar/runtime behavior.

## Current Risk Snapshot
- `spec/tools` currently fails compile in baseline (`CA_ANNOTATION` vs `ALT_ANNOTATION` drift).
- This demonstrates a missing hard contract between node type generation and tool consumption.

## Canonical Pipeline

```text
spec constructs/errors
  -> spec validators
  -> generated grammar corpus tests
  -> generated rust parser/validation tests
  -> generated error docs
  -> coverage dashboards and quality reports
```

## Structural Reorganization for `spec/tools`
- Split into explicit internal modules:
  - `input` (markdown/spec parsing)
  - `ir` (normalized internal representation)
  - `emit` (grammar tests, rust tests, docs)
  - `validate` (schema and semantic checks)
  - `sync` (grammar node-types and symbol-registry checks)

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

## Authoring Experience
- Provide strict but simple spec templates for constructs and errors.
- Add `spec lint` command for immediate feedback:
  - missing fields,
  - invalid tags,
  - malformed examples,
  - unknown error codes.

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
