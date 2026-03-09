# talkbank-parser

Parsing implementations for CHAT format using tree-sitter.

## Overview

This crate is the canonical parser for [CHAT format](https://talkbank.org/0info/manuals/CHAT.html),
converting source text into the `ChatFile` AST defined by
[talkbank-model](https://crates.io/crates/talkbank-model). It uses the
[tree-sitter-talkbank](https://github.com/TalkBank/tree-sitter-talkbank)
grammar to produce a concrete syntax tree (CST), then converts it into
the model's abstract types.

Key features:

- **Error recovery** — The GLR-based tree-sitter parser recovers from syntax
  errors and produces partial results, making it suitable for editor
  integration (LSP) and interactive use.
- **Thread-local parser pool** — Parser instances are reused via
  thread-local storage for efficiency.
- **Granular parsing** — The `ChatParser` trait allows parsing at any level:
  individual words, tiers, headers, or complete files.

This parser is validated against a 339-file reference corpus and must agree
with the direct parser on all well-formed inputs.

## Usage

```rust
use talkbank_parser::TreeSitterParser;
use talkbank_model::ChatParser;
use talkbank_model::ErrorCollector;

let parser = TreeSitterParser::new().expect("parser init");
let sink = ErrorCollector::new();
// parser.parse_chat_file(source) produces a ChatFile
```

## License

BSD-3-Clause. See [LICENSE](../../LICENSE) for details.

---

Implementation developed with [Claude](https://claude.ai) (Anthropic).
