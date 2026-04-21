# Rust Chatter vs. Java Chatter — Regression Audit

**Status:** Current
**Last updated:** 2026-04-20 19:30 EDT

## Purpose

Follow-on to `docs/reference-xml-coverage-gaps.md`. That doc
established that every missing XML golden corresponds to a Java
Chatter rejection. This doc answers the next question: **are any of
those rejections legitimate rules Rust dropped?**

Rust being *more permissive* than Java can mean two things:

- **Format evolution** — CHAT changed; Java is frozen; Rust is right.
- **Regression** — Rust silently accepts input a working check would
  still flag.

Below we audit each Java rejection category.

## Verdict summary

| Category | Java count | Verdict | Evidence |
|---|--:|---|---|
| `gra longer than mor` | 70 | Format evolution (cascade) | Rust has E720; doesn't fire because Rust parses the tokens Java skipped |
| `mor longer than main` | 53 | Format evolution (cascade) | Rust has E705/E706; doesn't fire for the same reason |
| `months for age must be two digits` | 56 | Format evolution | Rust matches CLAN CHECK 153 (stricter would exceed authority) |
| `nonviable character '‡'` | 15 | Format evolution | Post-Java CA notation |
| `no viable alternative at '|'` | 16 | Format evolution | CA/mor compound `‡|pos` |
| `no viable alternative at '-'` | 8 | Format evolution | Mor-feature dash-separation |
| `nonviable character ','` | 8 | Format evolution | Mor-feature comma-separation (`Int,Rel`) |
| `@Media present but no bullets` | 3 | **Candidate regression** | No Rust validator found |
| `"102" is not a legal word in language "eng"` | 1 | Format evolution | Java had a language word-list Rust intentionally doesn't |
| Other one-offs (ANTLR parser artifacts) | 15 | Format evolution | Various post-Java grammar additions |

**One candidate regression identified.** Every other Java diagnostic
is traceable to CHAT evolution.

## Detailed findings

### 1. Alignment cascades (70 + 53 = 123 lines) — NOT a regression

Java's "mor longer than main" and "gra longer than mor" errors
cascade from *lexer* failures earlier on the same line. When Java
encountered a post-Java character like `‡`, `,` (inside mor
features), or `-` (inside mor features), it dropped the character
and kept parsing. That made its token count wrong, which made the
alignment comparison fail.

Concrete example — `corpus/reference/languages/deu-conversation.cha`
line 9:

```
*MOT:   was isn da Caroline (.) .
%mor:   pron|was-Int,Rel-Nom-S1 verb|isnen-Fin-Ind-Pres-S2 ...
```

Java's error was:

```
line 9, column 19: lexer: skipping nonviable character ','
line 9, column 23: parser: no viable alternative at input '-'
line 9, column 31: parser: semantic failure: mor line is longer than main line
```

The cascade: `,` and `-` inside `was-Int,Rel-Nom-S1` are
modern mor-feature separators. Java skipped them → the mor-word
token `pron|was` was truncated → what followed ended up being parsed
as free-floating tokens → total token count inflated → alignment
rule fired.

Rust's view of the same line: `pron|was-Int,Rel-Nom-S1` is one
mor-word with dash- and comma-separated features. Token count is
right. Alignment is satisfied.

**Rust's equivalent alignment validators exist and are tested:**

| Rust code | Covers |
|---|---|
| E705 | `%mor` has too few tokens vs. main tier |
| E706 | `%mor` has too many tokens vs. main tier |
| E720 | `%gra` count mismatch vs. `%mor` |
| E714 / E715 | `%pho` count mismatch |
| E733 / E734 | `%mod` count mismatch |
| E725–E728 | Syllable tier count mismatches |
| E722 / E723 / E724 | `%gra` structural checks (ROOT, cycles) |

Spec files: `spec/errors/E70[5-6].md`, `E720_auto.md`, `E714_auto.md`,
`E715_auto.md`, `E73[3-4]_auto.md`, `E72[2-8]*.md`.

These fire correctly on genuinely misaligned tiers (confirmed by
`make verify`'s G7 / G8 gates). No regression.

### 2. Age-months zero-padding (56 lines) — NOT a regression

Java rejected `3;0` / `2;0` on every @ID line that used single-digit
months. The CLAN C tool — the **authoritative reference** — is more
lenient: CHECK error 153 ("Age's month or day are missing initial
zero") only fires when the age contains a `.` (days separator).
Single-digit months without a `.` like `3;0` are accepted.

Rust matches CLAN exactly:

```rust
// crates/talkbank-model/src/model/header/codes/age.rs
/// CLAN only checks this when the age contains a period (`.`), meaning
/// the days component is present. Without a period, single-digit months
/// like `2;6` are accepted. With a period, both month and day must be
/// two digits: `1;8.` → `1;08.`, `3;0.5` → `3;00.05`.
pub fn needs_zero_padding(&self) -> bool { ... }
```

Java Chatter was **stricter than CLAN**. It enforced two-digit
months unconditionally. The original CLAN implementation — the C
tool Leonid Spektor wrote — never did. Rust matching CLAN is the
correct behavior.

Source: `crates/talkbank-model/src/model/header/codes/age.rs:101-131`
(comment documents the CLAN CHECK 153 reference).

### 3. Post-Java CA / mor-feature syntax (47 lines) — NOT a regression

These are three closely-related Java failures:

- `‡` (U+2021 dagger) — 15 lines, all in `word-features/1082.cha`.
  Modern CA notation. Java's grammar doesn't know it.
- `no viable alternative at '|'` — 16 lines, co-occurring with `‡`
  above. The `‡|pos` pattern is a compound CA/mor marker.
- `no viable alternative at '-'` — 8 lines. Dash-separated mor
  features (`was-Int,Rel-Nom-S1`).
- `skipping nonviable character ','` — 8 lines. Comma-separated mor
  features (`Int,Rel` = interrogative-OR-relative).

All three are legitimate grammar evolution. The Rust tree-sitter
grammar (`grammar/grammar.js`) explicitly supports each: see the
`word_body` rule for CA notation and the `mor_feature` rule for
dash/comma-separated feature tokens.

No regression — Rust gained support for constructs Java never had.

### 4. `@Media` without bullets (3 lines) — **CANDIDATE REGRESSION**

This is the one finding where Rust appears to be missing a check.

Java rejected these three files:

- `corpus/reference/annotation/long-features.cha` — `@Media: long-features, audio`
- `corpus/reference/annotation/groups-sign.cha` — `@Media: groups-sign, video`
- `corpus/reference/tiers/sin.cha` — `@Media: sin, video`

With diagnostic:

```
semantic failure: not 'unlinked' or 'missing' or 'notrans',
but there are not bullets in transcript
```

The Java rule: if `@Media` is declared without a status value of
`unlinked` / `missing` / `notrans`, the transcript must contain
timing bullets (`·start_end·`). A "present but unlinked" declaration
requires explicit marking.

**Rust does not enforce this rule.** A search across the validation
codebase turns up no cross-check between `MediaHeader` (which
recognizes `Unlinked` / `Notrans` / `Missing` — see
`crates/talkbank-model/src/model/header/enums.rs:235-278`) and the
actual presence of bullets in utterance bodies. The
`crates/talkbank-model/src/validation/` module does not have a
dedicated media-presence validator.

#### Is the rule still valid in modern CHAT?

This needs domain confirmation. Arguments either way:

- **Still valid:** A declared `@Media` that lies about linkage is
  silent data corruption. CLAN CHECK likely still enforces this.
  The three affected files are test fixtures that probably *should*
  carry `@Media: name, audio, unlinked` to be honest about their
  status.
- **Relaxed:** Modern batchalign3 output, utterance-level timing
  (e.g., via `%wor` tier), and emerging conventions may make bullets
  no longer the only timing vehicle. If the rule is "must have
  *either* bullets *or* a timing sidecar", Rust needs the more
  sophisticated check — not the Java one.

**Action required:** confirm with Brian whether this rule should
exist, and in what form. If yes, it wants a new error spec
(tentatively E3xx — header semantics) plus a validator pass that
correlates header state with utterance-body state.

The three affected files should in the meantime continue to validate
clean; adding the check later will re-classify them as invalid (with
a suggested fix: add `, unlinked` to their `@Media` header).

### 5. Language-specific word dictionary (1 line) — NOT a regression

Java emitted one error:

```
*** File "fra-conversation.cha": ...
parser: semantic failure: "102" is not a legal word in language "eng"
```

Java Chatter shipped a per-language valid-word dictionary inherited
from CLAN's earlier behavior. This was always a partial/brittle
check (dictionaries lagged reality) and was superseded in later
CLAN versions by `CHECK -f` which only runs on request. Modern
chatter (both Rust and recent CLAN) does not perform compiled-in
word-list validation.

Intentional omission, not a regression.

### 6. Other one-off ANTLR artifacts (15 lines) — NOT a regression

Small numbers of `parser: no viable alternative at input '!'`,
`mismatched input '@Options:' expecting END`, and similar messages.
These are ANTLR recovery surface errors from Java's grammar
encountering post-Java constructs. All cascade from earlier
"nonviable character" skips or document-level header ordering that
Java's grammar hardcoded and modern CHAT relaxed.

Specific examples:

- `line 4, column 24: parser: required (...)+ loop did not match
  anything at input '!'` — Java's parser couldn't complete a
  required production after a prior skip.
- `line 1, column 34: parser: mismatched input '@Options:' expecting
  END` — Java required `@Options:` in a specific position; modern
  CHAT allows it more freely.

Rust parses these correctly.

## Recommendations

### Action items

1. **File the @Media/bullets check as an open question.** Open a
   documented issue (or table in `docs/`) asking Brian whether the
   rule should carry forward; if yes, author `spec/errors/E3NN.md`
   and implement a cross-check validator.

2. **Do not rush.** This is one candidate across 65 files and 245
   diagnostic lines. The regression surface is small and known.
   Successor-safety is best served by documenting the question now
   and resolving it deliberately, not by guessing.

3. **Keep the 00errors.cex files in tree.** They are the paper
   trail that made this audit possible and are useful every time
   someone adds to `corpus/reference/` — if a new `.cha` file has no
   golden, the first question is always "what did Java say?"

### Audit limitations

This audit looked at Java's *explicit* rejections (files that
appeared in `00errors.cex`). It does not cover the inverse regression
shape: **files Java accepted where its XML output reflects a
pre-evolution interpretation of a construct that modern CHAT now
reads differently.** That audit should happen when structural
mismatches surface during `%wor` / terminator / CA-notation
increments, not speculatively.

## Cross-references

- Gap inventory: `docs/reference-xml-coverage-gaps.md`
- Age-months logic: `crates/talkbank-model/src/model/header/codes/age.rs`
- Alignment validators: `crates/talkbank-model/src/alignment/` and
  spec files `spec/errors/E70*.md`, `E71*.md`, `E72*.md`, `E73*.md`
- Media header enum: `crates/talkbank-model/src/model/header/enums.rs`
- Verification gates: `Makefile` G7 (alignment) / G8 (roundtrip)
