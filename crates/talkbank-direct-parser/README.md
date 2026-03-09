# talkbank-direct-parser

Direct CHAT parser (non-CST), optimized for batch parsing.

## Overview

This crate provides an alternative parser for [CHAT format](https://talkbank.org/0info/manuals/CHAT.html)
that operates without producing a concrete syntax tree. Built with
[chumsky](https://crates.io/crates/chumsky) combinators, it is designed for
batch processing of well-formed input where error recovery is not needed.

Unlike the canonical [talkbank-parser](https://crates.io/crates/talkbank-parser),
this parser is fail-fast: it returns `Err` on the first error rather than
attempting recovery. This makes it faster for large-scale corpus processing
where all files are expected to be valid.

Both parsers implement the `ChatParser` trait from `talkbank-model` and must
agree on the reference corpus.

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
