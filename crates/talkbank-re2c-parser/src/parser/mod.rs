//! CHAT parser — re2c lexer + chumsky parser combinators.
//!
//! Entry points in `entry_points.rs`. Chumsky parsers in `main_tier.rs`,
//! `dependent_tiers.rs`, `headers.rs`. File-level parser in `file.rs`.

pub mod classify;
pub mod dependent_tiers;
pub mod entry_points;
pub mod file;
pub mod headers;
pub mod main_tier;
pub mod word_body;

// Re-export all public entry points so existing code can use `parser::parse_*`.
pub use entry_points::*;

use crate::lexer::Lexer;
use crate::token::Token;

/// Lex input text into a leaked token slice for chumsky parsers.
///
/// NUL-pads and leaks the input string, then collects lexer output into
/// a leaked slice. Both are leaked so `&'a [Token<'a>]` has a stable
/// lifetime for chumsky's `Input` trait.
pub(crate) fn lex_to_tokens<'a>(input: &str, start_condition: usize) -> &'a [Token<'a>] {
    let mut padded = input.to_string();
    padded.push('\0');
    let padded: &'a str = Box::leak(padded.into_boxed_str());
    let lexer = Lexer::new(padded, start_condition);
    let tokens: Vec<Token<'a>> = lexer.map(|(tok, _span)| tok).collect();
    Box::leak(tokens.into_boxed_slice())
}
