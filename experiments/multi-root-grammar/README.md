# Experiment: Multi-Root Grammar for CHAT Fragment Parsing

**Status:** In progress
**Last updated:** 2026-03-22 19:42 EDT

## Hypothesis

CHAT is line-oriented. Most external users want to parse **specific line
types** — not full documents. A single grammar with multiple entry points
(multi-root) would let tree-sitter parse fragments directly, eliminating the
need for synthetic document wrappers, Chumsky, or a custom fragment parser
generator.

## Motivation

External users of CHAT data (Phon project, independent researchers, Python
tools) typically want to parse:

| Fragment | Example | Use case |
|----------|---------|----------|
| Main tier line | `*CHI:\thello world .` | Utterance extraction |
| Dependent tier line | `%mor:\tv\|want qn\|more` | Morphology analysis |
| Header line | `@Participants:\tCHI Child` | Metadata extraction |
| Utterance block | `*CHI:\thello .\n%mor:\tv\|hello .` | Full utterance with tiers |
| Bare content | `hello world .` | Main tier content without speaker prefix |
| Mor word | `v\|want-PAST` | Single morphological analysis |
| Gra relation | `1\|2\|SUBJ` | Single grammatical relation |

Many users have "CHAT-ish" files that omit `@UTF8`, `@Begin`, `@End`, or
other structural headers. They just want to parse the lines they care about.

## Approach

Modify `grammar.js` to make the root rule a choice:

```javascript
document: $ => choice(
    $.full_document,     // existing full CHAT file
    $.utterance,         // main tier + dependent tiers (no headers)
    $.main_tier,         // single main tier line
    $.dependent_tier,    // single dependent tier line
    $.header,            // single header line
)
```

## What to Measure

1. **Does `tree-sitter generate` succeed?** (no new unresolvable conflicts)
2. **Conflict count change** (currently 5 — does it increase?)
3. **Parser binary size** (`parser.c` lines, compiled `.so` size)
4. **Parse speed** (benchmark on reference corpus — regression?)
5. **Full-document parsing still correct?** (reference corpus roundtrip)
6. **Fragment parsing works?** (parse individual lines)
7. **Editor integration** (VS Code highlighting/folding still works?)

## Scripts

```bash
# Run the full experiment
./run.sh

# Individual steps
./01-baseline.sh          # Measure current grammar
./02-modify-grammar.sh    # Create multi-root variant
./03-generate.sh          # tree-sitter generate on variant
./04-compare.sh           # Compare metrics
./05-test-fragments.sh    # Test fragment parsing
```

## Results (2026-03-22)

### Parser metrics

| Metric | Baseline | Multi-Root | Delta |
|--------|----------|-----------|-------|
| grammar.js lines | 2,045 | +~15 | Minimal |
| parser.c lines | 28,220 | 28,587 | +367 (+1.3%) |
| Rules | 367 | 368 | +1 (`full_document`) |
| Conflicts | 5 | 6 | +1 (flagged unnecessary) |
| Extras | 0 | 0 | Same |
| Reference corpus | 74/74 clean | Same rules | No regression |

### Fragment parsing: 45/45 clean

Every `@`, `*`, and `%` line type in the grammar parses correctly as a
standalone fragment:

- **30 header types** (`@Languages`, `@Participants`, `@ID`, `@Date`, etc.)
- **13 dependent tier types** (`%mor`, `%gra`, `%pho`, `%com`, etc.)
- **3 main tier variants** (`*CHI:`, `*MOT:`, `*INV:`)
- **1 multi-line utterance block** (main tier + dependent tiers)
- **1 full document** (unchanged behavior via `full_document`)

### Implications

1. **Eliminates synthetic document wrappers** — no more constructing fake
   `@UTF8\n@Begin\n...\n@End` documents to parse fragments
2. **Eliminates the need for Chumsky** (`talkbank-direct-parser`, 9,400 lines)
3. **Simplifies spec test generation** — specs can test fragments directly
4. **Simplifies LSP re-parsing** — parse changed lines, not synthetic documents
5. **Enables external tools** — Phon, researchers parse individual lines directly

### Recommendation

Make multi-root the default grammar. The cost (1.3% parser size, 1 precedence
declaration) is negligible. The benefit (fragment parsing for free, entire
categories of complexity removed) is transformative.
