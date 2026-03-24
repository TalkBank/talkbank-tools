# Library Usage

**Status:** Current
**Last updated:** 2026-03-21

The TalkBank Rust crates can be used as dependencies in your own Rust projects for parsing, validating, and manipulating CHAT files.

**Important:** some legacy tree-sitter fragment helpers are synthetic rather
than semantically honest. They can inject fragment input into boilerplate CHAT
text and parse the resulting synthetic file. Prefer full-file parsing for real
tree-sitter use, and do not treat legacy fragment helpers as the long-term
fragment API. For direct-parser fragment semantics, use direct-parser-native
tests instead of treating synthetic wrappers as the oracle.

## Adding Dependencies

Since the crates use path dependencies (not published on crates.io), add them as git or path dependencies:

```toml
[dependencies]
talkbank-model = { path = "../talkbank-tools/crates/talkbank-model" }
talkbank-transform = { path = "../talkbank-tools/crates/talkbank-transform" }
talkbank-parser = { path = "../talkbank-tools/crates/talkbank-parser" }
```

## Parsing and Validating a CHAT File

The simplest entry point is `parse_and_validate` from `talkbank-transform`:

```rust
use talkbank_transform::parse_and_validate;
use talkbank_model::ParseValidateOptions;

let source = std::fs::read_to_string("file.cha")?;
let options = ParseValidateOptions::default().with_validation();
let chat_file = parse_and_validate(&source, options)?;

for utterance in &chat_file.utterances {
    println!("Speaker: {:?}", utterance.speaker);
}
```

For batch workflows, reuse a parser instance:

```rust
use talkbank_parser::TreeSitterParser;
use talkbank_transform::parse_and_validate_with_parser;
use talkbank_model::ParseValidateOptions;

let parser = TreeSitterParser::new()?;
let options = ParseValidateOptions::default().with_validation();

for path in &chat_files {
    let source = std::fs::read_to_string(path)?;
    let chat_file = parse_and_validate_with_parser(&parser, &source, options)?;
    // ...
}
```

## Working with the Model

```rust
use talkbank_model::model::*;

// Access headers
let participants = &chat_file.headers.participants;

// Iterate utterances
for utt in &chat_file.utterances {
    // Access dependent tiers
    for tier in &utt.dependent_tiers {
        match tier {
            DependentTier::Mor(mor_tier) => {
                for item in &mor_tier.items {
                    println!("POS: {}, Lemma: {}",
                        item.main.pos, item.main.lemma);
                }
            }
            _ => {}
        }
    }
}
```

## Serializing to CHAT

```rust
use talkbank_model::WriteChat;

// WriteChat uses std::fmt::Write — write into a String directly
let chat_text = chat_file.to_chat_string();

// Or for streaming output:
let mut output = String::new();
chat_file.write_chat(&mut output)?;
```

## Serializing to JSON

```rust
let json = serde_json::to_string_pretty(&chat_file)?;
```

The JSON follows the schema at `schema/chat-file.schema.json`. For schema-validated JSON output, use `talkbank_transform::json::to_json_validated()`.

## Custom Error Handling

Implement `ErrorSink` for custom error handling:

```rust
use talkbank_model::{ErrorSink, ParseError};

struct MyErrorHandler;

impl ErrorSink for MyErrorHandler {
    fn report(&self, error: ParseError) {
        // Custom handling: log, filter, count, etc.
        eprintln!("[{}] {}", error.code, error.message);
    }
}
```

## Crate Selection Guide

| Need | Crate |
|------|-------|
| Data model types and error types | `talkbank-model` |
| Parse CHAT files (low-level) | `talkbank-parser` |
| Full pipeline (parse + validate + convert) | `talkbank-transform` |
| CLAN analysis commands | `talkbank-clan` |

## Batchalign3-Facing Surface

If you are building `batchalign3` or another external consumer, the stable
surface is usually:

| batchalign3 need | Prefer |
|------------------|--------|
| Canonical full-file parsing | `talkbank-parser` |
| Parse/validate contracts and typed model access | `talkbank-model` |
| Alignment-aware downstream consumers (`align`, `compare`, `benchmark`) | `talkbank-model` alignment helpers plus the model AST |
| Whole-pipeline parse+validate+convert | `talkbank-transform` |

For batch workflows, keep parser instances reusable and keep alignment logic
separate from parse semantics.

For CLAN analysis integration, prefer the library-owned execution boundary in `talkbank-clan` instead of constructing command types ad hoc in outer crates. In practice that means:

- use `talkbank_clan::framework::UtteranceRange` and `DiscoveredChatFiles` for analysis input selection
- parse raw outer-layer command names into `talkbank_clan::service::AnalysisCommandName` at the boundary
- use `talkbank_clan::service::AnalysisOptions` and `AnalysisRequestBuilder` when you need to translate raw outer-layer option bags into validated CLAN requests with library-owned defaults
- use `talkbank_clan::service::AnalysisRequest` to describe which CLAN analysis to run
- use `talkbank_clan::service::AnalysisService` when you need rendered or JSON analysis output from Rust code

That keeps CLI and editor integrations focused on adapting their own request shapes while the CLAN crate owns command construction and shared output behavior.
