# Spoken Content and Empty Spoken Content in CHAT Transcription

**Status:** Current
**Last updated:** 2026-04-04 08:37 EDT

## 1. Definition of "Spoken Content" in CHAT

CHAT transcription distinguishes between content that represents actual speech
produced by a speaker and structural/analytical markers that annotate or
categorize that speech. "Spoken content" is the phonological material that was
actually uttered (or intended to be uttered, in the case of omissions).

This distinction matters for:
- **Validation** (E209): a word token must have some lexical substance or be
  explicitly marked as untranscribed
- **Analysis** (FREQ, MLU, VOCD): only "countable" words contribute to metrics
- **Alignment** (%mor, %pho, %wor): empty words should not participate in
  cross-tier alignment
- **NLP pipelines** (batchalign): `cleaned_text()` feeds downstream models

## 2. How CLAN (C Code) Determines Spoken Content

CLAN does **not** have a unified concept of "spoken content." Instead, each
tool applies ad hoc string-prefix checks to decide what to include or exclude.

### CHECK Error 155: Standalone Shortening Outside CA Mode

**Location:** `OSX-CLAN/src/clan/check.cpp:4725-4729`

```c
if (*utterance->speaker == '*' || uS.partcmp(utterance->speaker,"%wor:",FALSE,FALSE)) {
    j = strlen(word) - 1;
    if (!check_isCAFound && j > 0 && word[0] == '(' && word[j] == ')') {
        if (!uS.isPause(word, 1, NULL, NULL) && strchr(word+1, '(') == NULL)
            check_err(155,s-1,te-1,ln+lineno);
    }
```

**What this checks:** On main tier or %wor tier, outside CA mode, if a "word"
starts with `(` and ends with `)`, and it is not a pause marker, and there is
no nested `(` inside, then error 155 fires.

**Error message:** `Please use "0word" instead of "(word)".`

This is a narrow check: it catches standalone shortenings like `(the)` used as
words, telling the transcriber to use `0the` instead. It does NOT check for
general "empty spoken content" -- it checks for a specific mis-notation pattern.

### CLAN Analysis Tools: String Prefix Exclusions

CLAN's `FREQ`, `MLU`, `VOCD`, and other tools exclude words using raw string
checks:

| String pattern | What it catches | Intent |
|---|---|---|
| `word[0] == '#'` | Pauses (`#`, `#300`) | Not a word |
| `word[0] == '+'` | Terminators (`+...`, `+//.`) | Not a word |
| `word == "xxx"` | Unintelligible | No lexical content |
| `word == "yyy"` | Needs phonetic coding | No lexical content |
| `word == "www"` | Deliberately untranscribed | No lexical content |
| `word[0] == '0'` | Omitted words (`0is`) | Absent speech |
| `word[0] == '&'` | Fillers/nonwords/fragments | Not lexical items |

### Key Observation: CLAN Has No "Spoken Material" Concept

CLAN never asks "does this word contain spoken material?" as a general
question. Instead:
- CHECK error 155 catches one specific notation mistake
- Analysis tools skip words by prefix category
- There is no equivalent of our `has_spoken_material()` in the C code

### Java Chatter (`java-chatter-stable/`)

The Java chatter **does** validate spoken content, in `ChatParser.g:2276-2277`:

```java
if (!$wn.hasContent && !this.allowUnspokenContent) {
    error(wd, "word has no spoken content");
}
```

The `hasContent` flag tracks whether a `wordNet` contains any
`wordWithProsodies` (actual text/prosodic content) vs. being composed entirely
of shortening. This is exactly the same concept as our `has_spoken_material()`.

The `allowUnspokenContent` flag is set to `true` when `@Options: CA` or
`@Options: CA-Unicode` is present (`ChatParser.g:706`). This means standalone
shortenings like `(the)` are allowed in CA mode (where `((word))` notation is
used for inaudible speech) but flagged as errors in normal mode.

The `expectShortening` rule (`ChatParser.g:2681`) returns `hasContent = false`
when the word is ONLY shortenings with no `wordWithProsodies`, and
`hasContent = true` when there is text adjacent to the shortening (e.g.,
`(be)cause` has text "cause" so `hasContent = true`).

This confirms our E209 implementation is correct and matches both CLAN CHECK
155 and the Java chatter's "word has no spoken content" error.

## 3. How Our Rust Code Determines Spoken Content

Our implementation has **two distinct but related concepts**:

### 3.1. `has_spoken_material()` -- Word-Internal Content Check

**Location:** `crates/talkbank-model/src/validation/word/structure.rs:347-356`

```rust
fn is_spoken_material(content: &WordContent) -> bool {
    matches!(content, WordContent::Text(text) if !text.as_ref().is_empty())
}

pub(crate) fn has_spoken_material(word: &Word) -> bool {
    word.content.iter().any(is_spoken_material)
}
```

**What it checks:** Iterates the word's `content` vector and returns `true` if
any element is a `WordContent::Text` variant with non-empty text.

**What counts as spoken material:**
- `WordContent::Text` with non-empty content: **YES** -- this is the ONLY thing
  that counts

**What does NOT count:**
- `WordContent::Shortening` -- omitted sounds, e.g. `(be)` in `(be)cause`
- `WordContent::StressMarker` -- prosodic annotation
- `WordContent::Lengthening` -- prosodic annotation
- `WordContent::SyllablePause` -- prosodic annotation
- `WordContent::OverlapPoint` -- structural CA marker
- `WordContent::CAElement` -- CA prosodic marker
- `WordContent::CADelimiter` -- CA prosodic marker
- `WordContent::UnderlineBegin/End` -- formatting control
- `WordContent::CompoundMarker` -- structural metadata
- `WordContent::CliticBoundary` -- structural metadata

**Critical design choice:** `Shortening` is NOT spoken material for
`has_spoken_material()`, even though `Shortening` IS included in
`cleaned_text()`. This means a word that is purely a standalone shortening
(like `(the)` in non-CA mode) has `has_spoken_material() == false` but
`cleaned_text() == "the"`.

### 3.2. `cleaned_text()` -- NLP-Ready Lexical Text

**Location:** `crates/talkbank-model/src/model/content/word/word_type.rs:285-295`

```rust
pub fn compute_cleaned_text(&self) -> String {
    let mut result = String::new();
    for item in &self.content {
        match item {
            WordContent::Text(t) => result.push_str(t.as_ref()),
            WordContent::Shortening(s) => result.push_str(s.as_ref()),
            _ => {}
        }
    }
    result
}
```

**What it includes:** `Text` + `Shortening` (restoring elided material)

**What it excludes:** All prosodic markers, CA markers, overlap points,
compound markers, clitic boundaries, underline markers

**Examples:**
- `sit(ting)` -> `sitting` (Text "sit" + Shortening "ting")
- `bana:nas` -> `bananas` (Text "bana" + Lengthening + Text "nas")
- `ice+cream` -> `icecream` (Text "ice" + CompoundMarker + Text "cream")
- `(be)cause` -> `because` (Shortening "be" + Text "cause")
- `(the)` -> `the` (Shortening "the" only -- cleaned_text is non-empty but
  has_spoken_material is false)

### 3.3. E209 Trigger Logic

**Location:** `crates/talkbank-model/src/model/content/word/word_validate.rs:81-96`

```rust
if !structure::has_spoken_material(self) && self.untranscribed().is_none() {
    errors.report(/* E209: EmptySpokenContent */);
}
```

E209 fires when BOTH:
1. `has_spoken_material()` returns `false` (no `WordContent::Text` elements)
2. `untranscribed()` returns `None` (not `xxx`/`yyy`/`www`)

### 3.4. `is_countable_word()` -- Analysis-Level Filtering

**Location:** `crates/talkbank-clan/src/framework/word_filter.rs:51-72`

This is a higher-level concept used by CLAN analysis commands. A word is NOT
countable if:
- `untranscribed()` is `Some` (xxx, yyy, www)
- `category` is `Omission`, `Filler`, `Nonword`, or `PhonologicalFragment`
- `cleaned_text()` is empty

Note: `is_countable_word()` uses `cleaned_text()` (which includes shortenings),
while `has_spoken_material()` uses only `WordContent::Text` (which excludes
shortenings). This is a meaningful difference.

## 4. Complete Word Content Type Classification

| WordContent Variant | In `cleaned_text()`? | In `has_spoken_material()`? | Description |
|---|---|---|---|
| `Text(non-empty)` | Yes | **Yes** | Plain spoken text segments |
| `Shortening(text)` | Yes | **No** | Omitted sound in parens, e.g. `(be)` |
| `StressMarker` | No | No | Primary (ˈ) or secondary (ˌ) stress |
| `Lengthening` | No | No | Syllable lengthening (:) |
| `SyllablePause` | No | No | Pause between syllables (^) |
| `OverlapPoint` | No | No | CA overlap markers within words |
| `CAElement` | No | No | Individual CA prosodic markers |
| `CADelimiter` | No | No | Paired CA prosodic markers |
| `UnderlineBegin` | No | No | Control character for underline |
| `UnderlineEnd` | No | No | Control character for underline |
| `CompoundMarker` | No | No | Word-internal compound boundary (+) |
| `CliticBoundary` | No | No | Morphological clitic marker (~) |

### Word-Level Markers (Not WordContent)

These are properties of the `Word` struct, not content elements:

| Marker | Effect on spoken status | Effect on countability |
|---|---|---|
| `category: Omission` (0word) | Has text -> has_spoken_material | NOT countable |
| `category: Filler` (&-um) | Has text -> has_spoken_material | NOT countable |
| `category: Nonword` (&~gaga) | Has text -> has_spoken_material | NOT countable |
| `category: PhonologicalFragment` (&+fr) | Has text -> has_spoken_material | NOT countable |
| `category: CAOmission` ((word)) | Has text -> has_spoken_material | Countable |
| `form_type` (@l, @c, @d, etc.) | Does not affect spoken material check | Does not affect countability |
| `lang` (@s:eng) | Does not affect spoken material check | Does not affect countability |
| `untranscribed` (xxx/yyy/www) | has_spoken_material = true (text exists) | NOT countable |

## 5. Edge Cases

### Words That Are Only Form Markers (`@l`, `@s:eng`)

A bare `@l` or `@s:eng` with no preceding word body is a parser-level issue.
The parser currently treats `@l` as a valid word `""` with form type `L`. If
the parser produces a Word with empty content, `has_spoken_material()` returns
false and E209 should fire. However, the E209 spec notes this does not
currently trigger because `@l` is parsed as a complete special-form word.

**Open question:** Should `b@l` (letter "b" with @l marker) have spoken material?
Yes -- the `b` is a `Text` element, so `has_spoken_material()` returns true.

### Shortening-Only Words: `(the)`

- `has_spoken_material()` = **false** (only `Shortening`, no `Text`)
- `cleaned_text()` = `"the"` (non-empty)
- `is_countable_word()` = **true** (cleaned_text non-empty, no exclusion category)
- In non-CA mode: CLAN CHECK 155 fires ("use 0the instead")
- In CA mode: parsed as `CAOmission`, has `Text` content, no error

**Inconsistency:** A standalone `(the)` in non-CA mode passes `is_countable_word()`
but fails `has_spoken_material()`. This is correct -- E209 fires during
validation to flag the notation error, while `is_countable_word()` is defensive
about not double-excluding during analysis.

### Zero-Words (`0word`)

- `has_spoken_material()` = **true** (the word body "word" is a `Text` element)
- `category` = `Omission`
- `is_countable_word()` = **false** (omission category)

Zero-words have spoken material in the sense that there IS text content in the
word model, but they are not countable because they represent absent speech.
E209 does NOT fire for zero-words because `has_spoken_material()` is true.

### Untranscribed Markers (`xxx`, `yyy`, `www`)

- `has_spoken_material()` = **true** (the marker text itself is a `Text` element)
- `untranscribed()` = `Some(...)` (classified)
- `is_countable_word()` = **false** (untranscribed status)
- E209 does NOT fire (explicitly exempted: `&& self.untranscribed().is_none()`)

### Fillers (`&-um`)

- `has_spoken_material()` = **true** (text "um" exists)
- `category` = `Filler`
- `is_countable_word()` = **false** (filler category)
- E209 does NOT fire
- `cleaned_text()` = `"um"`

### Fragments (`&+fri`)

- `has_spoken_material()` = **true** (text "fri" exists)
- `category` = `PhonologicalFragment`
- `is_countable_word()` = **false** (fragment category)
- E209 does NOT fire
- `cleaned_text()` = `"fri"`

### Nonwords (`&~laugh`)

- `has_spoken_material()` = **true** (text "laugh" exists)
- `category` = `Nonword`
- `is_countable_word()` = **false** (nonword category)
- E209 does NOT fire
- `cleaned_text()` = `"laugh"`

### Retraced Content (`<word> [/]`)

Retrace groups are handled at the `UtteranceContent` level, not the `Word`
level. The word inside a retrace group is a normal `Word` with spoken material.
The retrace annotation affects **alignment** (skipped for %mor domain) and
**analysis** (skipped by `countable_words` unless `include_retracings` is true),
but does not affect the word's own `has_spoken_material()` result.

### Replaced Content (`word [: replacement]`)

The original word (before `[:`) has its own `has_spoken_material()` -- it is a
normal spoken word. The replacement words inside `[:]` are validated separately
via `replacement.rs:149`, which also checks `has_spoken_material()` and reports
E209 if a replacement word has no spoken content.

### Words With Only Prosodic Markers

A word like `:` (just lengthening) or `ˈ` (just stress) would have no `Text`
elements, so `has_spoken_material()` = false. These would trigger E209. In
practice, the parser is unlikely to produce such words because prosodic markers
must attach to text.

### Compound Words With Empty Parts

`ice+` (trailing compound marker) or `+cream` (leading compound marker) would
trigger E232/E233 (compound marker position errors). The non-empty part still
has `Text`, so `has_spoken_material()` is true.

## 6. Divergence Between CLAN and Our Implementation

| Aspect | CLAN (C) | Our Rust Code |
|---|---|---|
| **Error 155 / E209 scope** | 155 checks only standalone `(word)` outside CA mode | E209 checks any word with no `Text` content and no untranscribed marker |
| **Conceptual model** | No unified concept; ad hoc string checks per tool | Typed AST with `has_spoken_material()` as a single predicate |
| **Shortening treatment** | 155 catches `(word)` as a whole-string pattern | `has_spoken_material()` excludes `Shortening` from "spoken" |
| **"Empty word" detection** | No general empty-word check exists in CLAN | E209 is a general structural check |
| **Analysis exclusions** | String prefix matching (`word[0] == '0'`, etc.) | Category enum matching + `is_countable_word()` |

**Our E209 is broader than CLAN's error 155.** CLAN 155 catches one specific
mistake (standalone shortening in non-CA mode). Our E209 catches any word that
has no `WordContent::Text` elements, regardless of what other content is
present. This is a stricter, more principled check.

## 7. What E209 (EmptySpokenContent) Should Actually Validate

E209's current logic is correct in intent: a word token should have actual
lexical text content or be an explicit untranscribed marker. The two conditions
(`!has_spoken_material(self) && self.untranscribed().is_none()`) correctly
express this.

**The spec example is the problem, not the code.** The E209 spec file uses
`@l` as an example, but the parser treats `@l` as a complete word. To trigger
E209, a spec example needs a word that the parser produces with no `Text`
content elements. Possible triggers:

1. A parser recovery case where error nodes consume all text but leave the word
   shell
2. A word constructed from only prosodic markers (unlikely from real input)
3. A standalone shortening `(word)` in non-CA mode -- this WOULD trigger E209
   because the word's content has only `Shortening`, no `Text`

**Recommendation for the spec:** Replace the `@l` example with a standalone
shortening like `(word)` in non-CA mode. This is the most realistic trigger
and matches CLAN's error 155 semantics. Example:

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
*CHI:	(the) dog .
@End
```

This should produce E209 (no `Text` content in the word `(the)`) and is the
direct analog of CLAN error 155.

## 8. Summary of the Three Layers

```
Layer 1: has_spoken_material()  -- Word-internal structural check
  "Does this word have any Text content elements?"
  Used by: E209 validation, prosodic marker placement checks (E245, E246, E252)

Layer 2: cleaned_text()  -- NLP text extraction
  "What is the lexical text of this word, with shortenings restored?"
  Includes: Text + Shortening
  Used by: JSON serialization, tier alignment, analysis input

Layer 3: is_countable_word()  -- Analysis-level filtering
  "Should this word be counted in linguistic analyses?"
  Excludes: untranscribed, omissions, fillers, nonwords, fragments, empty text
  Used by: FREQ, MLU, VOCD, COOCCUR, KIDEVAL, SCRIPT
```

These three layers are intentionally different. A word can have
`cleaned_text()` non-empty but `has_spoken_material()` false (standalone
shortening). A word can have `has_spoken_material()` true but
`is_countable_word()` false (filler, omission). The layered design reflects the
real semantic distinctions in CHAT transcription.
