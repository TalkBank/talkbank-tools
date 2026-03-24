# Architecture Audit

**Status:** Current
**Last updated:** 2026-03-23 23:49 EDT

This page records the current internal architecture audit for `talkbank-tools`.
It focuses on present structure and follow-up refactor targets, not on the
historical sequence of refactors that led here.

Longer-horizon ideas that are not active work should go on the
[Rearchitecture Backlog](rearchitecture-backlog.md).

## Current Strengths

- alignment and traversal behavior are now more explicit: `walk_content` and
  `walk_words` are clearer seams than the older mixed walker surface
- overlap handling now has a better model for cross-utterance and 1:N cases,
  with a corresponding debug/audit command instead of one-off investigation
  paths
- language header handling is stricter and more explicit, especially around
  `@ID` language values
- the repo already has strong documentation around coding standards, boundary
  vocabulary, and testing expectations

## Active Findings

### 1. Alignment logic still spans too many layers

The overlap/alignment redesign improved the model, but the related logic still
touches CLI, LSP, parser tests, and model helpers at once. The architecture is
clearer than before, but the coordination cost is still high.

Desired direction:

- keep the alignment domain centered in `talkbank-model`
- keep CLI/LSP as adapters over that domain instead of carrying parallel local
  logic
- prefer fewer, named overlap/alignment entrypoints over more helper-shaped
  surfaces

### 2. Validation and spec drift remains a standing risk

This review reconciled many stale `not_implemented` and dead error specs, but
that cleanup was large enough to confirm the process risk: specs, code, and
generated/schema artifacts can still drift for too long before being audited.

Desired direction:

- treat spec drift as a routine maintenance axis rather than a periodic sweep
- keep "implemented vs documented vs generated" in one verification loop
- keep deletion of dead codes/specs normal instead of exceptional

### 3. Type discipline improved, but boundary conversion is not done

`LanguageCode`/`LanguageCodes` handling is better, and the repo already has
good anti-primitive-obsession guidance, but string-heavy seams still exist
around header parsing and some validation/reporting boundaries.

Desired direction:

- convert to domain types earlier
- keep parser output and validation input closer to the typed model
- avoid backsliding into raw strings for convenience in CLI/editor adapters

### 4. Testing breadth is strong but expensive

The parser snapshot surface and spec corpus give the repo unusually good
coverage, but they also make large refactors noisy and raise the cost of
regeneration decisions.

Desired direction:

- keep coverage broad, but make regeneration workflows more explicit
- separate intentional snapshot churn from architectural regressions
- keep the debug/audit tools close to the areas they validate

### 5. ~~Direct-parser fragment testing~~ (Resolved)

The direct parser (Chumsky) has been removed. Tree-sitter is the sole parser.
All concerns about direct-parser fragment oracles, parity gates, and
bootstrap-era testing are now obsolete.

### 6. Spec/generation system cleanup

The split of `spec/tools` and `spec/runtime-tools` into separate crates is
complete. With the direct parser removed, the spec/generation system now
targets tree-sitter only, eliminating the bootstrap-era circular coupling.

Remaining direction:

- separate grammar corpus generation, full-file parity tests, and
  validation/error specs into distinct tracks
- narrow `spec/tools` to real artifact generation and spec validation
- replace large generation rituals with smaller workflows matched to the kind
  of change being made

Current progress:

- `spec/tools` and `spec/runtime-tools` are now separate crates in the spec
  workspace
- `spec/tools` is back to core generation concerns
- runtime-aware mining/validation tooling now lives in `spec/runtime-tools`,
  with Makefile/docs updated accordingly
