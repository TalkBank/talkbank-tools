# Parser Error Recovery: Direct Parser vs Tree-Sitter Parser

**Status:** Reference document
**Date:** 2026-03-21

This document audits the real current recovery behavior of both parsers, compares their leniency, and maps the parse/validate boundary for each.

**Important:** fragment input is still a first-class concept in TalkBank tooling, specs, and tests. What is *not* first-class is the old assumption that tree-sitter fragment helpers are honest isolated-fragment parsers. Several of those helpers wrap fragment input in boilerplate CHAT text and then parse the whole wrapped file. That synthetic behavior must no longer be treated as the semantic oracle for direct-parser fragment behavior.

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Error Recovery Architecture](#2-error-recovery-architecture)
3. [Blast Radius Comparison](#3-blast-radius-comparison)
4. [Word Parsing: The Critical Case](#4-word-parsing-the-critical-case)
5. [Dependent Tier Recovery](#5-dependent-tier-recovery)
6. [Parse/Validate Boundary](#6-parsevalidate-boundary)
7. [Leniency Comparison](#7-leniency-comparison)
8. [Current Gaps and Cleanup Targets](#8-current-gaps-and-cleanup-targets)

---

## 1. Executive Summary

The two parsers still have materially different recovery strategies, but the older description of the direct parser as simply "fail-fast" is now wrong.

| Property | Tree-Sitter Parser | Direct Parser |
|----------|-------------------|---------------|
| **Recovery source** | GLR CST recovery with `ERROR`/`MISSING` nodes during full-file parse | Hand-owned recovery in main-tier, dependent-tier, grouping, and file paths |
| **Fragment honesty** | Full-file parsing is real; several fragment helpers are synthetic wrappers | Fragment parsing is real, but semantics differ by entry point |
| **Isolated `parse_word()`** | Synthetic wrapper behavior | Still strict: malformed words reject |
| **Main-tier malformed-word handling** | Often preserves partial word via CST recovery | Now preserves malformed token as raw-text `Word` inside the tier |
| **Dependent tier recovery** | Often item-level within a tier | Tier-level drop plus parse-health taint; later valid siblings can survive |
| **File-level recovery** | Always yields a recovered `ChatFile` | Can recover degraded main tiers and malformed dependent tiers if structure survives |
| **Dummy fabrication** | Can fabricate conservative words from `ERROR` text in some paths | Should not fabricate placeholder tiers; uses raw-text preservation instead |

**Bottom line:** full-file tree-sitter parsing remains the most forgiving structural recovery path, but the direct parser is no longer accurately described as “one bad word kills the file.” The real gap now is subtler:

- direct fragment semantics need their own oracle
- tree-sitter fragment helpers are still synthetic in important places
- some direct-parser recovery is still too silent, especially when malformed words are preserved as raw text without surfacing the inner word diagnostics

---

## 2. Error Recovery Architecture

### 2A. Tree-Sitter Parser: CST-Guided Recovery

Tree-sitter's GLR parser always produces a full CST for a full file. Invalid regions become `ERROR` nodes and missing required elements become `MISSING` nodes. The Rust layer then walks that CST and attempts to preserve as much model structure as possible.

This is real and trustworthy for **full-file** parsing.

It is **not** equally trustworthy as an isolated-fragment story. Several tree-sitter fragment helpers work by injecting the fragment into a boilerplate CHAT file and then extracting the relevant node back out. That makes them useful legacy audits, but not honest isolated-fragment parsers.

Important recovery mechanisms in the Rust layer:

- `ERROR` node analysis for user-facing diagnostics
- conservative `ERROR`-as-word recovery in some main-tier word paths
- attachment of compact error fragments to the preceding word in specific tree paths
- item-skip behavior inside some dependent tiers

### 2B. Direct Parser: Selective Recovery Owned In Rust

The direct parser uses chumsky, but its current behavior is no longer “pure fail-fast.” Recovery now exists at multiple levels:

- malformed words inside **main tiers** can be preserved as raw-text `Word` values
- malformed replacement words can also be preserved as raw text
- malformed dependent tiers can be dropped while parse-health taint is still classified
- degraded main-tier shells can survive file parsing when a speaker code is recoverable
- later valid dependent tiers can still attach after an earlier malformed sibling tier

The important distinction is that direct-parser recovery is **entry-point specific**:

- `parse_word()` is still strict
- `parse_main_tier()` is more lenient
- `parse_utterance()` and `parse_chat_file()` are more lenient still

That is a real semantic contract and should be documented and tested directly instead of inferred from legacy tree-sitter fragment behavior.

---

## 3. Blast Radius Comparison

### What Happens When One Thing Goes Wrong

| Error Location | Tree-Sitter Parser | Direct Parser |
|---------------|-------------------|---------------|
| **Malformed word in isolated word helper** | Synthetic wrapper behavior; useful only as a legacy audit | `parse_word()` rejects |
| **Malformed word inside main tier** | Often preserves a word via CST recovery | Main-tier parser can preserve the token as raw text |
| **Malformed `@s:` marker inside word** | Often keeps the word and reports a diagnostic | Depends on entry point; strict `parse_word()` rejects, tier-level parsing may preserve raw text |
| **Unknown annotation** | Grammar/CST dependent | Explanation fallback exists in direct main-tier parsing |
| **One bad MOR/GRA/PHO item** | Often item-level skip inside tier | Whole tier usually dropped, parse-health tainted |
| **Malformed dependent tier label or structure** | File survives with recovered structure | Tier dropped, alignment domains tainted, later valid siblings can remain |
| **Malformed main tier with recoverable speaker code** | File survives | File parser can build degraded main-tier shell |
| **Malformed header / file structure** | Tree-sitter still yields a CST | Direct file parser can still reject if structure phase fails |

### Visual Summary

```
Tree-sitter full-file recovery:
  bad local region      -> CST keeps going -> converter salvages structure -> file survives

Direct parser current recovery:
  bad isolated word     -> word rejects
  bad word in main tier -> raw-text word preserved
  bad dependent tier    -> tier dropped, health tainted, file/utterance can survive
  bad main tier body    -> degraded shell may survive if speaker can be recovered
```

---

## 4. Word Parsing: The Critical Case

This is still the most important comparison, but the question is now more nuanced than it was in February.

### 4A. Tree-Sitter Word Parsing

Full-file tree-sitter parsing remains structurally richer and more forgiving. The grammar decomposes words into fine-grained nodes, and CST conversion can sometimes preserve a word even when one sub-component is malformed.

That remains valuable for full-file parsing and whole-file equivalence audits.

It does **not** mean tree-sitter fragment helpers are the oracle for fragment semantics, because those helpers may be parsing boilerplate-wrapped mini files rather than honest isolated words.

### 4B. Direct Parser Word Parsing

The direct parser now has two distinct contracts:

- `parse_word()` remains strict and rejects malformed words
- `parse_main_tier()` can preserve malformed word-like tokens as raw-text `Word`s so one bad token does not kill the whole tier

That difference is intentional enough that it must be documented as a contract, not treated as an implementation accident.

### 4C. Concrete Example: `he(llo`

| Entry Point | Current Direct-Parser Behavior |
|------------|-------------------------------|
| `parse_word("he(llo")` | Rejects |
| `parse_main_tier("*CHI:\thello he(llo world .")` | Preserves `he(llo` as raw-text `Word` |
| `parse_utterance(...)` | Inherits main-tier recovery behavior |

This is one of the clearest examples of why fragment semantics need a direct-parser-native test oracle.

---

## 5. Dependent Tier Recovery

The big current difference is still **granularity**.

### 5A. Tree-Sitter: Often Item-Level Granularity

When tree-sitter plus CST conversion can isolate the malformed local region, a tier may remain partially populated.

### 5B. Direct Parser: Mostly Tier-Level Granularity

The direct parser usually handles malformed dependent tiers by:

- dropping the tier
- preserving the surrounding utterance or file
- tainting the relevant alignment domain in parse health
- keeping later valid sibling tiers when possible

That is materially better than the old “dependent-tier failure poisons everything” model, but it is not yet full item-level recovery parity.

### 5C. Parse-Health Classification

The direct parser's `classify_dependent_tier_parse_health()` remains important because it can taint the correct alignment domain even when the tier itself could not be fully parsed.

That keeps downstream alignment behavior honest:

- malformed `%mor` should not silently permit main↔mor alignment
- malformed `%gra` should not silently permit mor↔gra alignment
- malformed unknown tiers should conservatively taint alignment-dependent domains

---

## 6. Parse/Validate Boundary

### What Each Layer Is Responsible For

```
┌─────────────────────────────────────────────────────┐
│  GRAMMAR / STRUCTURE                                │
│  • tree-sitter full-file structure and CST recovery │
│  • chumsky file/tier structure in direct parser     │
└──────────────────────┬──────────────────────────────┘
                       ▼
┌─────────────────────────────────────────────────────┐
│  PARSER LAYER                                        │
│  • CST → model conversion (tree-sitter)              │
│  • fragment/tier/content parsing (direct parser)     │
│  • parse-health taint tracking                       │
│  • local recovery / raw-text preservation            │
│  • degraded-shell recovery                           │
└──────────────────────┬──────────────────────────────┘
                       ▼
┌─────────────────────────────────────────────────────┐
│  VALIDATION LAYER                                    │
│  • cross-tier consistency                            │
│  • header requirements                               │
│  • speaker/participant checks                        │
│  • CA balance and temporal invariants                │
│  • alignment checks gated by ParseHealth             │
└─────────────────────────────────────────────────────┘
```

The key boundary reminder is:

- **Parsing** decides what structure and degraded recovery artifacts exist
- **Validation** decides whether that recovered structure is semantically acceptable

Neither layer should silently invent “clean” meaning where the input was malformed.

---

## 7. Leniency Comparison

### Where Tree-Sitter Is Still More Forgiving

1. Whole-file structural recovery remains stronger.
2. Dependent-tier recovery is often finer-grained inside a tier.
3. Full-file CST recovery keeps more local failure context available.

### Where The Direct Parser Is Now More Flexible Than The Old Docs Claimed

1. Main-tier malformed-word recovery now preserves raw text instead of always killing the tier.
2. Degraded main-tier shells can survive file parsing.
3. Malformed dependent tiers can be dropped without discarding later valid sibling tiers.
4. Context-specific annotation fallback exists in main-tier parsing.

### Where The Direct Parser Is Still Too Weak Or Too Silent

1. Isolated `parse_word()` is stricter than main-tier parsing, and that contract needs explicit test coverage.
2. Recovered raw-text words currently preserve content but can lose the inner word diagnostics that justified the recovery.
3. Dependent-tier recovery is still mostly tier-level instead of item-level.
4. Too many tests still compare against synthetic tree-sitter fragment behavior instead of asserting direct-parser semantics directly.

---

## 8. Current Gaps and Cleanup Targets

The highest-value next work is now:

1. keep full-file tree-sitter equivalence tests, but stop treating tree-sitter fragment helpers as the oracle
2. build direct-parser-native fragment specs for malformed-word recovery, degraded-shell behavior, and non-fabrication guarantees
3. make synthetic wrapper behavior explicit everywhere in code and docs
4. decide whether isolated `parse_word()` should stay strict or grow its own explicit recovery contract
5. eliminate silent recovery where malformed content is preserved but its diagnostics disappear
6. only after the spec/test harness is corrected, narrow or retire the legacy public tree-sitter fragment convenience surface

---

## See Also

- [parsing.md](parsing.md) — parser overview and synthetic-fragment warning
- [spec-system.md](spec-system.md) — how fragment specs are wrapped for generated tree-sitter tests
- [grammar-redesign.md](grammar-redesign.md) — grammar coarsening discussion
- [leniency-policy.md](leniency-policy.md) — parser/validation permissiveness policy
- `crates/talkbank-direct-parser/src/file/mod.rs` — file parsing and degraded-shell recovery
- `crates/talkbank-direct-parser/src/main_tier/mod.rs` — main-tier recovery behavior
- `crates/talkbank-direct-parser/src/main_tier/words.rs` — raw-text word recovery paths
- `crates/talkbank-parser/src/parser/chat_file_parser/single_item/` — synthetic tree-sitter fragment helpers
