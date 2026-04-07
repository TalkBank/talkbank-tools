# Alignment Divergence Report: talkbank-tools vs Python batchalign

**Date:** 2026-02-12
**Status:** Historical / superseded by current `%wor` policy

---

## Executive Summary

This report records where the Rust validator/generator diverged from legacy
Python batchalign `%wor` output. Python batchalign remains useful as forensic
evidence about historical output, but it is **not** the semantic authority for
alignment rules: validation is defined by the TalkBank alignment spec, which
keeps `%wor` membership deterministic and alignment itself strictly 1:1.

---

## Purpose

This report is retained as **forensic history** about legacy Python batchalign
output. It should not be used as the current `%wor` specification.

Legacy Python batchalign behaved as if:

1. fragments, nonwords, and `xxx`/`yyy`/`www` were excluded from `%wor`
2. replacement text displaced the original spoken word in `%wor`

The current TalkBank policy intentionally does **not** follow those rules.

## Current Policy That Supersedes This Report

Current `%wor` semantics are defined by the TalkBank alignment spec and the
cross-repo rollout tests, not by Python batchalign's historical lexer behavior.

Current `%wor`:

- counts spoken word tokens, including fillers, fragments, nonwords, and
  `xxx`/`yyy`/`www`
- uses the **original spoken surface** for replacements
- treats retrace as traversal only, not as a special membership override
- remains strictly **1:1** once membership is determined

So if legacy Python batchalign output omits a spoken token from `%wor`, that is
historical evidence about old output, not evidence that the current validator
should be lenient.

### 1. Error Code Sharing (E714/E715)

%wor alignment reuses error codes E714 (`PhoCountMismatchTooFew`) and E715
(`PhoCountMismatchTooMany`) from %pho. The error messages include "%wor tier"
in the text, so this is functional. Consider dedicated codes if needed.

### 2. Terminator Validation

%mor validates terminator consistency (E707). %wor does not have an equivalent
check. Consider adding if needed.

---

Last Updated: 2026-02-12
