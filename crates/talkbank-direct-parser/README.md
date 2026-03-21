# talkbank-direct-parser

Direct CHAT parser (non-CST), optimized for batch parsing.

## Overview

This crate provides an alternative parser for [CHAT format](https://talkbank.org/0info/manuals/CHAT.html)
that operates without producing a concrete syntax tree. Built with
[chumsky](https://crates.io/crates/chumsky) combinators, it started as a
strict fragment parser but now includes selective recovery and leniency in
some file and tier paths.

Unlike the canonical [talkbank-parser](https://crates.io/crates/talkbank-parser),
this parser does not build a CST. Recovery, where present, is explicit and
hand-owned rather than inherited from a GLR parse. That means it needs its own
test oracle for fragment and recovery semantics; synthetic tree-sitter
fragment helpers should not be treated as the source of truth.

For fragment work, prefer direct-parser-native tests that state the recovery
contract explicitly. Legacy synthetic fragment paths exist only for audit or
compatibility checks.

Both parsers implement the `ChatParser` trait from `talkbank-model` and must
agree on the reference corpus for full-file behavior.

## Usage

```rust
use talkbank_direct_parser::DirectParser;

let parser = DirectParser::new().expect("parser init");
// parser.parse_chat_file(source) returns Result<ChatFile, ParseErrors>
```

## License

BSD-3-Clause. See [LICENSE](../../LICENSE) for details.

---

Implementation developed with [Claude](https://claude.ai) (Anthropic).
