# Parser, Model, and API Contracts

## Current Strength
`talkbank-model` (in its `parser_api` module) already defines a broad `ChatParser` trait for multiple parse granularities.
This is a strong foundation.

## Current Gaps
1. Result and error behavior can vary by callsite.
2. Canonical vs fragment/parser-role boundaries are documented but not always enforced structurally.
3. Integrator contract guarantees are not centralized in a strict compatibility policy document.

## Batchalign3-Facing Contract

`batchalign3` relies on these guarantees from `talkbank-tools`:

- parsing produces a typed `ChatFile` or an explicit parse-status signal
- parse-health taint is visible to alignment consumers
- alignment helpers operate on semantic model types, not raw text hacks
- recovery never fabricates valid-looking placeholder semantics for malformed input

That means the parser/model boundary must stay honest enough for downstream
workflows like `align`, `compare`, `benchmark`, and morphotagging to make
their own validity decisions.

## Canonical Contract Model

### Public Contract Layers
1. Parse API Contract:
  - stable function signatures,
  - deterministic parse result envelope,
  - clear partial-success semantics.
2. Semantic Model Contract:
  - stable core model fields,
  - explicit unstable/internal fields policy.
3. Diagnostic Contract:
  - stable error code IDs and severity semantics,
  - best-effort message text compatibility.
4. Serialization Contract:
  - deterministic output constraints,
  - normalized formatting policy.

### Required Types
- `ParseOutcome<T>`
  - `value: T | omitted-by-status`
  - `diagnostics: Vec<Diagnostic>`
  - `status: Success | Partial | Failed`
- `Diagnostic`
  - `code`, `severity`, `category`, `message`, `location`, `context`, `suggestion`

## Parser Role Enforcement
- `talkbank-parser`: production default for CLI/LSP/API.
- `talkbank-direct-parser`: explicit fragment and alternate full-file parser with
  its own documented recovery/leniency contract.
- CLI must surface parser selection as an advanced/debug option, not a default user burden.

## Invariants
1. Parsing with offset must shift all spans consistently.
2. Parse-level and validation-level diagnostics must remain distinguishable.
3. Serialization should preserve semantic equivalence and documented formatting rules.
4. Roundtrip behavior must be testable per parser implementation.
5. Parser functions that accept `ErrorSink` should not return `Option<T>` for fallible parse state.

## API Versioning Policy (Pre-Release but Strict)
- Even before 1.0, publish internal `CONTRACT_LEVELS.md`:
  - Stable-for-integrators
  - Stable-internal
  - Experimental
- Mark every public function/type by contract level.

## Acceptance Criteria
- Single canonical parse outcome envelope exposed for integrators.
- Parser implementations conform to shared contract tests.
- Contract-level annotations exist for all public API surfaces.
- Documentation for parse/validate/serialize lifecycle is centralized and current.

## Recovery Contract: No Fabricated Semantic Values
The parser contract must forbid sentinel semantic values during error recovery.

Disallowed recovery behavior:
- returning arbitrary enum variants as fallback for unknown/missing nodes,
- returning empty strings as stand-ins for required fields,
- constructing fake words/chunks like `"missing"`, `"error"`, or other placeholders.

Required recovery behavior:
1. Emit structured diagnostic with precise span and expected node kind.
2. Return an explicit parse-status signal (`Partial`/`Failed`) through `ParseOutcome`.
3. Omit invalid semantic node OR store it in explicit recovery metadata, never as a valid semantic value.

Current enforcement:
- CI guardrail script tracks and blocks introduction of new `ErrorSink + Option` signatures.
- See `scripts/check-errorsink-option-signatures.sh` and `scripts/errorsink_option_allowlist.txt`.

Rationale:
- fabricated semantic values create secondary, misleading diagnostics against synthetic data,
- downstream tools cannot distinguish real user content from parser-generated placeholders,
- equivalence and regression tests become noisy and non-actionable.

For `batchalign3`, this is especially important because alignment workflows
must be able to tell the difference between:

- a malformed input that should taint or block alignment
- a recoverable input where raw text can be preserved
- a clean input that should proceed through the align/compare pipeline

## String Storage Policy

The model uses three string storage strategies:
- **`Arc<str>` interning** (`interned_newtype!`): For high-frequency repeated values (POS tags, stems, speaker codes). Global interner avoids redundant allocations.
- **`SmolStr`** (`string_newtype!`): For short strings (median 10–15 chars) that benefit from inline storage. O(1) clone, no heap allocation for strings ≤23 bytes.
- **`String`**: Only for utility types outside the core model (e.g., `semantic_diff/`).
