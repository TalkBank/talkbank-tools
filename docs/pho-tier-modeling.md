# Phonological Tier Data Modeling: Current Status and Simplifications

**Status:** Current
**Last modified:** 2026-03-30 16:27 EDT

## Summary

The `%pho` and `%mod` tiers in CHAT transcripts support IPA phonological
transcription. The Phon project (phon.ca) created a rich structured XML
representation of these tiers in the TalkBank XML schema, with typed elements
for phonetic words, syllables, stress, sandhi, prosody, and pauses. Our current
Rust data model intentionally simplifies this to opaque phonetic word strings
with compound markers and grouping, deferring full Phon-level structural
modeling to a future phase.

## What the Phon XML Schema Defines

The TalkBank XML schema (`talkbank-xml-schema/src/main/resources/talkbank.xsd`)
defines `phoneticTranscriptionType` for `%pho` and `%mod`:

```xml
<xs:complexType name="phoneticTranscriptionType">
  <xs:choice minOccurs="0" maxOccurs="unbounded">
    <xs:element ref="pw"/>      <!-- phonetic word -->
    <xs:element ref="pause"/>   <!-- timed/untimed pause -->
    <xs:element ref="phog"/>    <!-- phonological group (‹...›) -->
  </xs:choice>
</xs:complexType>
```

A phonetic word (`pw`) is itself structured:

```xml
<xs:complexType name="phoneticWord">
  <xs:choice minOccurs="1" maxOccurs="unbounded">
    <xs:element ref="stress"/>  <!-- primary/secondary stress -->
    <xs:element ref="ph"/>      <!-- phone (single IPA segment) -->
    <xs:element ref="cmph"/>    <!-- compound marker (+) -->
    <xs:element ref="pp"/>      <!-- prosody (syllable break, etc.) -->
    <xs:element ref="sandhi"/>  <!-- sandhi linking -->
  </xs:choice>
</xs:complexType>
```

### Key Phon structures NOT modeled in our Rust types:

1. **Phones (`ph`)** — individual IPA segments with optional diacritic
   annotations. The XML represents each phoneme as a separate element.
   We store the entire word as an opaque IPA string.

2. **Syllable structure** — the XML's `pp` elements mark syllable
   boundaries (`.`) as typed prosody elements. We include dots in the
   opaque pho_word string.

3. **Stress typing** — the XML's `stress` elements are typed as
   `primary` or `secondary`. We carry stress in the IPA string notation
   (ˈ, ˌ) without structured typing.

4. **Sandhi** — the XML has `sandhi` elements for phonological linking
   between words. We don't model this.

5. **Pauses in pho tiers** — the XML treats `pause` as a peer of `pw`
   (phonetic word) inside phonetic transcriptions. Our grammar.js
   includes `(`, `.`, `)` in the `pho_word` regex, so `(..)` is parsed
   as a phonological word, not a structured pause. This is intentional —
   the Phon pause semantics are preserved in the string content, just not
   as a typed model element.

## What We Currently Model

### Grammar level (grammar.js)

```
pho_groups → pho_group (ws pho_group)*
pho_group  → pho_words | ‹ pho_grouped_content ›
pho_words  → pho_word (+ pho_word)*
pho_word   → /[IPA chars including (.)*^]+/
```

The `pho_word` regex is deliberately permissive — it includes parentheses,
dots, carets, and asterisks. This means `(..)` is a valid pho_word, not a
separate pause construct.

### Data model level (talkbank-model)

```rust
pub struct PhoTier {
    pub tier_type: PhoTierType,    // Pho or Mod
    pub items: Vec<PhoItem>,
}

pub enum PhoItem {
    Word(PhoWord),                  // Opaque IPA string
    Group(PhoGroup),                // ‹ words › grouping
    Terminator(PhoTerminator),      // . ? !
}

pub struct PhoWord {
    pub text: SmolStr,              // Full IPA text including dots, parens
    pub compounds: Vec<SmolStr>,    // Split on +
}
```

The `PhoWord.text` carries the full IPA string including syllable dots,
parenthesized pauses, stress markers — everything. No substructure.

### Re2c lexer level

The `PHO_CONTENT` condition produces:
- `PhoWord` — any IPA character sequence (includes `.`, `(`, `)`)
- `PhoPlus` — compound marker `+`
- `PhoGroupBegin`/`PhoGroupEnd` — `‹` `›`
- `Whitespace` — word boundaries

No pause tokens, no terminator tokens, no stress tokens. Everything is
either a pho_word or structural (plus, grouping, whitespace).

## Why This Simplification

1. **Phon integration is in progress.** The full Phon XML model is
   complex (phones, syllables, stress, sandhi, alignment). Modeling it
   fully requires the Phon project's conversion tools and alignment
   algorithms. We defer this to when PhonTalk integration is complete.

2. **Roundtrip fidelity.** The opaque string model roundtrips perfectly —
   `%pho: wɑ+kɪŋ (.) hɛloʊ .` serializes back identically. A structured
   model would need to reconstruct the exact original notation.

3. **Grammar consistency.** Grammar.js uses a single permissive
   `pho_word` regex. Splitting it into typed tokens (pause vs word vs
   stress) would require grammar changes and complex disambiguation.

4. **The Phon-specific tiers (%xmodsyl, %xphosyl, %xphoaln) are
   separately modeled** with syllabified structure. These use
   `TIER_CONTENT` (text with bullets) and have dedicated model types.

## Phon Tiers in the Model

| Tier | Grammar condition | Model type | Structured? |
|------|-------------------|------------|-------------|
| `%pho` | PHO_CONTENT | `PhoTier` (opaque words) | Partial (compounds, grouping) |
| `%mod` | PHO_CONTENT | `PhoTier` (opaque words) | Partial (compounds, grouping) |
| `%xmodsyl` | TIER_CONTENT | `SylTier` | Yes (syllabified) |
| `%xphosyl` | TIER_CONTENT | `SylTier` | Yes (syllabified) |
| `%xphoaln` | TIER_CONTENT | `PhoalnTier` | Yes (alignment pairs) |

## Future Work

When Phon integration reaches the point where structured phonetic data
flows through the pipeline:

1. Add `PhoPhone`, `PhoSyllable`, `PhoStress` types to the model
2. Create a structured `PhoWord` that carries phone-level detail
3. Add `PhoPause` as a peer of `PhoWord` in `PhoItem` (matching the XSD)
4. Update grammar.js to split `pho_word` into substructure tokens
5. Update both parsers (TreeSitter and re2c)
6. Ensure roundtrip fidelity with the new structured representation

Until then, the opaque string model is correct and complete for all
existing corpus data.

## References

- TalkBank XML Schema: `talkbank-xml-schema/src/main/resources/talkbank.xsd`
- Phon manual: https://www.phon.ca/phon-manual/transcription/ipa_transcription.html
- CHAT manual phonological tiers: https://talkbank.org/0info/manuals/CHAT.html#Phonological_Coding
- Grammar.js pho rules: `grammar/grammar.js` lines 1840-1885
