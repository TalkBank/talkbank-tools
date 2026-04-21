# Reference-XML Golden Coverage Gaps

**Status:** Reference
**Last updated:** 2026-04-20 19:30 EDT

## Purpose

`corpus/reference-xml/` holds TalkBank-XML goldens produced by the
legacy Java Chatter tool and used as the parity oracle for the Rust
XML emitter in `crates/talkbank-transform/src/xml/`. The reference
CHAT corpus at `corpus/reference/` has 98 files; only 33 of them have
a paired `.xml` golden. This document explains the 65-file gap, so
that a successor reading the emitter harness does not mistake
Java-side rejections for Rust emitter bugs.

**Bottom line: the asymmetry is entirely Java-side.** Every missing
golden corresponds 1-to-1 with a file the frozen Java Chatter parser
rejected. Rust chatter validates all 98 reference files cleanly. CHAT
has evolved since Java Chatter development ceased; the missing
goldens are evidence of that evolution, not evidence of Rust
emitter defects.

## Methodology

Three artifacts establish the mapping:

1. **`corpus/reference-xml/**/*.xml`** — the 33 goldens Java produced.
2. **`corpus/reference/**/00errors.cex`** — per-subdirectory logs
   containing every diagnostic Java emitted while attempting to
   process the reference corpus. 245 error lines across 8 files.
3. **`chatter validate corpus/reference/ --force`** — the Rust
   validator's verdict on the same 98 files: all 98 valid, 0 invalid.

### Mapping procedure

```bash
# .cha files with no .xml golden (65 files)
comm -3 \
    <(cd corpus/reference && find . -name '*.cha' | sed 's|\.cha$||' | sort) \
    <(cd corpus/reference-xml && find . -name '*.xml' | sed 's|\.xml$||' | sort)

# Unique files Java rejected (65 files — same set)
cat corpus/reference/*/00errors.cex \
    | grep -oE 'File "[^"]+\.cha"' \
    | sort -u | sed 's/File "//;s/"$//'
```

The two sets match exactly: every gap is a Java rejection; no
Java-rejected file has a golden; no golden is missing for any
Java-accepted file.

## Java Chatter rejection categories

Count of unique diagnostic messages across the 245 error lines:

| Count | Java diagnostic | Root cause |
|------:|-----------------|-----------|
| 70 | `semantic failure: gra line is longer than mor line` | Downstream cascade — Java's lexer skipped tokens, leaving %mor/%gra misaligned to the truncated main tier |
| 56 | `semantic failure: months for age must be two digits` | CHAT format evolution — modern CHAT accepts `3;0` (single-digit months); Java's parser demanded `3;00` |
| 53 | `semantic failure: mor line is longer than main line` | Same cascade as the gra/mor case |
| 16 | `no viable alternative at input '|'` | CA/mor compound marker (`‡|pos`) that post-dates Java's grammar |
| 15 | `lexer: skipping nonviable character '‡'` | Dagger (U+2021) — modern CA notation, post-Java |
| 8 | `no viable alternative at input '-'` | German/multi-language separator combinations post-Java |
| 8 | `lexer: skipping nonviable character ','` | Same context — `,-` sequences Java's grammar cannot tokenize |
| 3 | `not 'unlinked' or 'missing' or 'notrans'` | `@Media` attribute values expanded post-Java |
| 16 | other (one-off tokenizer / parser rejections) | Various construct-level evolutions |

The three dominant families — age-months, alignment cascades, and
post-Java Unicode/punctuation — account for 230 of 245 lines (94%).

## Implications for emitter-parity work

### What the 33 present goldens cover

Java accepted these files, so their goldens represent the intersection
of "modern CHAT" and "Java-Chatter-era CHAT". Parity work against
them is valid: any structural mismatch between Rust and Java output on
these files is a real Rust emitter issue (or a documented schema
evolution that should be justified in the emitter's comments).

### What the 65 missing goldens do *not* cover

The Rust emitter has no external oracle for files like:

- `audio/french-child-speech.cha`, `audio/russian-child-narrative.cha`
- `content/words-basic.cha`, `content/words-prosody.cha`, …
- `word-features/1082.cha`, `word-features/000829.cha`
- `tiers/mor-gra.cha`, `tiers/pho.cha`, `tiers/wor.cha`
- Most of the `languages/` directory (German, Dutch, French, Russian…)

These need an alternative testing strategy:

1. **Roundtrip self-consistency** — `chat → xml → chat` equivalence
   via the structural comparator.
2. **Hand-authored goldens** reviewed by Brian MacWhinney or another
   domain expert familiar with the schema.
3. **Deferred** — acknowledged as untested and documented in the
   emitter's per-feature increment log.

### Latent audit item

A file Java accepted may still have a golden that reflects a
*pre-evolution* interpretation — e.g., a construct Java parsed one
way that modern CHAT now reads differently. The 33 goldens are not
automatically safe: they are safe for the constructs Java understood.
When a structural mismatch on a present golden points at a
construct-level disagreement, the right first question is "has this
construct's semantics changed since Java was frozen?" before
assuming a Rust emitter defect.

A future pass should cross-reference each present golden against the
CHAT manual's change history to flag pre-evolution interpretations
proactively.

## What this gap analysis is *not*

It is not a regression audit. Rust's permissiveness relative to Java
could in principle mean:

- **Legitimate format evolution** (the claim above — CHAT changed)
- **Regression** (Rust dropped a check Java had, and is now silently
  accepting malformed input)

Most of the diagnostic families above are clearly the first — age
digits, dagger CA notation, and new separator combinations all align
with documented schema evolution. But the cascading alignment errors
(mor/main, gra/mor) are *rules*, not *tokens*. Those rules must still
exist in Rust chatter. Investigating whether they do, and whether
they fire on the same semantic violations Java caught, is a separate
audit — see follow-on doc `docs/rust-vs-java-chatter-regressions.md`
(pending).

## Regenerating the goldens

The XML goldens were produced by running Java Chatter
(`chatter.jar`, historical build) against each `.cha` file and
capturing the `.xml` output alongside the `00errors.cex` diagnostic
log. Because Java Chatter is frozen, regeneration will produce
byte-identical output; the only mutation lever is adding new files to
the reference corpus, in which case any new Java-rejected entries
will add to the gap documented here.

The Rust emitter test harness lives at
`crates/talkbank-parser-tests/tests/xml_golden.rs` and compares
against whatever `.xml` files exist in `corpus/reference-xml/` —
missing goldens silently skip, matching the policy documented above.
