# Symbols

**Status:** Reference
**Last updated:** 2026-03-24 00:01 EDT

CHAT uses a rich set of symbols for transcription conventions. This page documents the symbol categories and the symbol registry that drives both the grammar and the Rust crates.

## Symbol Registry

The authoritative symbol definitions live in `spec/symbols/symbol_registry.json`. This JSON file is the single source of truth — it generates:

- Character sets for the tree-sitter grammar (`grammar.js`)
- Rust constants for the model and validation crates
- Validation rules for the spec tool

After any change to the symbol registry, run:

```bash
make symbols-gen
```

## Symbol Categories

### Terminators

Punctuation that ends an utterance:

| Symbol | Name | Usage |
|--------|------|-------|
| `.` | Period | Declarative |
| `?` | Question | Interrogative |
| `!` | Exclamation | Exclamatory |
| `+...` | Trailing off | Incomplete utterance |
| `+/.` | Interruption | Speaker interrupted by another |
| `+//.` | Self-interruption | Speaker interrupts self |
| `+/?` | Interrupted question | Question interrupted |
| `+!?` | Broken question | Exclamation-question |
| `+".` | Quoted new line | Quotation continues on next line |

### CA (Conversation Analysis) Delimiters

Symbols used in conversation analysis notation:

| Symbol | Meaning |
|--------|---------|
| `↑` | Rising pitch |
| `↓` | Falling pitch |
| `→` | Level pitch |
| `↗` | Rise-fall |
| `↘` | Fall-rise |
| `≋` | Creaky voice |
| `∙` | Micropause |

### CA Elements

Symbols that appear within conversation analysis annotations (prosodic marking, stress, etc.).

### Word Segment Characters

Characters that are forbidden at the start of words, forbidden in the rest of words, or forbidden throughout. These define the lexical boundaries of what constitutes a "word" in CHAT.

The grammar uses these sets to construct the word-matching regex patterns. Characters like `[`, `]`, `<`, `>`, `(`, `)` are structural delimiters and cannot appear inside words.

### Event Segment Characters

Characters forbidden in event descriptions (`&=event` content). Events have slightly different lexical rules than words.

## Language Codes

CHAT uses ISO 639-3 three-letter language codes in `@Languages` headers and `@s:` word markers:

```chat
@Languages:	eng, fra
*CHI:	I want a croissant@s:fra .
```

Common codes: `eng` (English), `fra` (French), `deu` (German), `spa` (Spanish), `zho` (Mandarin), `jpn` (Japanese).

## Special Markers

### @ Markers (Word-Level)

| Marker | Meaning |
|--------|---------|
| `@s:LANG` | Second language word |
| `@l` | Letter |
| `@c` | Child-invented form |
| `@f` | Family-specific word |
| `@n` | Neologism |
| `@o` | Onomatopoeia |
| `@b` | Babbling |
| `@wp` | Word play |
| `@si` | Signed word |

### & Markers (Events and Fillers)

| Prefix | Meaning |
|--------|---------|
| `&=` | Paralinguistic event (e.g., `&=laughs`) |
| `&-` | Filler (e.g., `&-um`) |
| `&*` | Interposed event |

### Scope Markers

| Marker | Meaning |
|--------|---------|
| `[/]` | Retrace (repetition) |
| `[//]` | Retrace with correction |
| `[///]` | Reformulation |
| `[/-]` | False start |
| `[*]` | Error |
| `[?]` | Best guess |
| `[>]` | Overlap follows |
| `[<]` | Overlap precedes |
| `[= text]` | Explanation |
| `[: text]` | Replacement |
