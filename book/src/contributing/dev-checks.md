# Developer Verification Checks

This page defines the canonical local verification gates that must pass before opening or merging a PR.

## Canonical Command
Run:

```bash
make verify
```

This runs 11 gates (G0–G10). See [Testing > Verification Gates](testing.md#verification-gates) for the full table. Key gates include:

- **G0** Parser signature guardrail
- **G1** Rust workspace compile check
- **G2** Spec tools compile check
- **G4** Generated parser corpus equivalence suite
- **G5** Word-level parser equivalence suite
- **G7** Reference corpus semantic equivalence (tree-sitter vs direct)
- **G9** Golden tier roundtrip (%mor, %gra, %pho, %wor)
- **G10** Reference corpus node coverage

## When to Run
- Always before creating a PR.
- Always before merging parser, spec-tool, or generated artifact changes.
- After rebasing if upstream changed parser/spec-tooling.

## Additional Engineering Checks

Run these in addition to `make verify` when touching parser/model code:

1. `cargo fmt` from repo root (use `cargo fmt`, not direct `rustfmt`).
2. `cargo test -p talkbank-parser --test test_parse_health_recovery`.
3. `cargo test -p talkbank-direct-parser --test test_parse_health_recovery`.
4. `cargo nextest run -p talkbank-parser-tests --test parser_equivalence_files`.

These protect against regressions in:
- parser recovery without sentinel fabrication
- parse-health taint propagation
- cross-parser semantic equivalence

## Failure Policy
- If any gate fails, do not merge.
- Fix the failing gate or scope down the change.
- If the failure is unrelated and pre-existing, document it in the PR and open a blocker issue.

## Recommended Fast Loop During Development
Use narrower loops while iterating, then run `make verify` before final review:

```bash
cargo test -p talkbank-direct-parser --lib
cargo nextest run -p talkbank-parser-tests --test parser_equivalence_words
```
