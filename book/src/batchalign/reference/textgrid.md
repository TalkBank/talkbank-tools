# TextGrid Format and Conversion

**Status:** Current
**Last updated:** 2026-05-21 08:43 EDT

---

## What is TextGrid?

**TextGrid** is a file format used by [Praat](https://www.fon.hum.uva.nl/praat/), a popular tool for phonetic analysis and speech research.

### Purpose

TextGrid files allow researchers to:
- Visualize waveforms and spectrograms alongside transcriptions
- Measure acoustic properties (formants, pitch, duration, etc.)
- Annotate speech at multiple levels (word, phone, syllable, etc.)
- Align transcriptions with audio for detailed phonetic analysis

### Structure

A TextGrid consists of **tiers** (one per speaker or annotation layer), where each tier contains **intervals**:

```text
Interval {
    xmin: 1.000     # Start time (seconds)
    xmax: 1.200     # End time (seconds)
    text: "hello"   # Label text
}
```

Example TextGrid with two speakers:

```text
File type = "ooTextFile"
Object class = "TextGrid"

xmin = 0.0
xmax = 10.5
tiers? <exists>
size = 2

item [1]:
    class = "IntervalTier"
    name = "CHI"
    xmin = 0.0
    xmax = 10.5
    intervals: size = 5
    intervals [1]:
        xmin = 0.0
        xmax = 1.2
        text = "hello"
    intervals [2]:
        xmin = 1.2
        xmax = 1.8
        text = "world"
    ...

item [2]:
    class = "IntervalTier"
    name = "MOT"
    xmin = 0.0
    xmax = 10.5
    intervals: size = 3
    ...
```

Both long (the example above) and short TextGrid formats are
supported by the parser.

---

## TextGrid Conversion in talkbank-tools

TextGrid ↔ CHAT conversion is implemented in Rust, inside the
`talkbank-clan` crate, and exposed through the `chatter` CLI.

### Entry points

| Direction | Rust function | Location |
|-----------|---------------|----------|
| TextGrid → CHAT | `praat_to_chat(content)` and `praat_to_chat_with_options(...)` | `crates/talkbank-clan/src/converters/praat2chat.rs:199`, `:204` |
| CHAT → TextGrid | `chat_to_praat(chat)` | `crates/talkbank-clan/src/converters/praat2chat.rs:292` |

Both functions operate on the typed `ChatFile` AST and return /
accept TextGrid strings; no intermediate Python layer is involved.

### CLI

The conversions are wired into `chatter clan` as the
`praat2chat` and `chat2praat` subcommands (dispatch in
`crates/talkbank-cli/src/commands/clan/converters.rs`):

```bash
# Convert a TextGrid file to CHAT
chatter clan praat2chat input.TextGrid > output.cha

# Convert a CHAT file to TextGrid
chatter clan chat2praat input.cha > output.TextGrid
```

### Programmatic use (Rust)

```rust,ignore
use talkbank_clan::converters::praat2chat::{praat_to_chat, chat_to_praat};

let chat_file = praat_to_chat(textgrid_content)?;
let textgrid_text = chat_to_praat(&chat_file)?;
```

---

## Dependencies

- **`talkbank-clan` crate**: owns the TextGrid parser, serializer,
  and converters.
- **No Python runtime dependency**: the previous Python
  implementation (`batchalign/formats/textgrid/generator.py` and the
  `batchalign_core.extract_timed_tiers` PyO3 binding) was retired
  when the converter moved into Rust. The `praatio` package still
  appears in `pyproject.toml` for unrelated Python tooling, but the
  TextGrid pipeline no longer routes through it.

---

## Reference fixtures

A canonical short-format example lives at
`crates/talkbank-clan/tests/fixtures/sample.TextGrid` and is used by
the round-trip unit tests in `praat2chat.rs` (see the
`#[test]` block starting at
`crates/talkbank-clan/src/converters/praat2chat.rs:415::praat_to_chat_basic`).
