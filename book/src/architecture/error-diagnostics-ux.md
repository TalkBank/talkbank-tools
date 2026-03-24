# Error System and Diagnostics UX

**Status:** Current
**Last updated:** 2026-03-24 00:01 EDT

## Objective
Make diagnostics precise, explainable, and actionable for both developers and non-technical editors,
while keeping machine readability for downstream tools.

## Existing Observations
- Extensive error code surfaces and many generated error docs already exist.
- Some current compile failures indicate type-level drift around location handling.
- Message quality is not yet governed by one central style standard.

## Canonical Diagnostic Schema

```text
Diagnostic {
  code: String,
  severity: Error | Warning | Info,
  category: Parse | Validation | Alignment | Header | Tier | Internal,
  location: SourceLocation,
  context: ErrorContext,
  message: String,
  suggestion: Option<String>,
  related: Vec<RelatedLocation>
}
```

## Message Quality Standard
Each diagnostic must answer:
1. What failed.
2. Where it failed.
3. Why it likely failed.
4. What to do next.

Avoid internal jargon unless accompanied by user-facing explanation.

## Severity Policy
- `Error`: blocks parse/validation outcome.
- `Warning`: content is usable but has quality/compliance concerns.
- `Info`: optional guidance and migration hints.

Severity must not be overloaded for tooling convenience.

## Recovery Policy: Diagnostic-First, Not Sentinel-First
When parser recovery is required:
- do not invent semantic fallback values to keep type construction convenient,
- do not use empty strings or arbitrary enum defaults as recovered content.

Instead:
1. report diagnostic with expected/actual node context,
2. preserve span information for tooling and UI,
3. propagate partial/failure status explicitly.

Any synthetic placeholders that are unavoidable for internal plumbing must be:
- non-semantic (not exposed as real model content),
- marked internal-only,
- excluded from user-facing diagnostics and serialization.

### Sentinel vs Error Variant Rule
If an unexpected condition changes semantic trust in parsed content:
- represent that explicitly as an error-bearing state (enum variant, parse-taint flag, or explicit outcome type),
- never represent it as `None` or a default payload that can be mistaken for valid content.

This applies both to parser outputs and to runtime metadata consumed during validation.

### Diagnostic Construction Standard
Use shared constructors/helpers for common diagnostics to reduce drift:
- span-only diagnostics (`code + severity + span + message`),
- source-backed diagnostics (`code + severity + span + source + offending + message`).

Benefits:
- consistent location/context population,
- fewer ad hoc `ParseError::new(...)` call shapes,
- simpler migration to richer miette rendering.

## Error Code Governance
- Central registry file under `talkbank-model` (errors module).
- One authoritative description and example per code.
- Deprecated codes remain mapped with explicit migration notes.
- CI check forbids duplicate code definitions or orphaned docs.

## Span and Location Correctness
- All diagnostics must use consistent line/column and byte offset definitions.
- Add golden tests for:
  - single-byte and multi-byte UTF-8 content,
  - embedded content offsets,
  - continuation lines and tabs.

## Integrator Output Formats
Provide:
- human-readable CLI diagnostics,
- machine-readable JSON diagnostics,
- LSP diagnostic mapping.

All formats must share the same underlying diagnostic schema.

## Acceptance Criteria
- Every emitted diagnostic includes code, severity, location, and suggestion policy.
- Error code documentation and runtime definitions are synchronized automatically.
- Span correctness is covered by dedicated tests.
- CLI and JSON outputs are contract-tested for schema compliance.
