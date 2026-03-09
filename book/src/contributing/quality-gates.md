# Testing and Quality Gates

> **Note:** This page was originally a gap analysis from 2026-02-18. Most gaps identified have since been fixed — CI now runs all G0–G10 gates. See [Testing](testing.md) for current gate definitions.

## Verified Current Baseline
1. Local verification is strong and currently green:
   - `make check` passes.
   - `make verify` passes through G0-G9.
2. Root CI exists and is functional in `.github/workflows/ci.yml`:
   - `rust-check-and-test`
   - `spec-tools`
   - `grammar`
   - `generated-artifacts`
3. Drift protection exists and is active:
   - `make generated-check` is run in CI.
   - parser signature guardrail is enforced in CI.

## Gap: Local vs CI Contract Mismatch
CI still runs a narrower bar than local `make verify`.

### Covered in CI
1. Compile checks (`cargo check`, `spec/tools cargo check`)
2. Core crate tests (`talkbank-model`, `talkbank-parser`, `talkbank-parser-tests --lib`)
3. Grammar generation and grammar corpus tests
4. Generated artifact regeneration check
5. Formatting and clippy

### Not Covered in CI (But Covered by `make verify`)
1. Generated parser corpus equivalence suite (`G4`)
2. Word-level parser equivalence (`G5`)
3. Bare timestamp regression (`G6`)
4. File-level parser equivalence (`G7`)
5. `%wor` terminator/alignment gate (`G8`)
6. Golden tier roundtrip parser suite (`G9`)
7. Reference corpus roundtrip gate (`roundtrip_corpus`)

## Improvement Plan
1. Promote `make verify` into CI as a required job, or replicate G4-G9 explicitly in CI jobs.
2. Add a required reference corpus roundtrip job (cache-friendly by default, forced mode on schedule/nightly).
3. Fail `spec/tools` warnings for binaries that are intended to be maintained (or suppress intentionally with explicit rationale).
4. Add machine-readable gate summary artifact per CI run to simplify regression triage.

## Acceptance Criteria For This Area
1. CI gate set is a superset of local pre-merge policy, not a subset.
2. Parser behavioral regressions (equivalence, `%wor`, golden tier) fail PRs before merge.
3. Reference corpus roundtrip status is visible in CI and release decisions.
4. `spec/tools` check is warning-clean or has documented intentional exceptions.
