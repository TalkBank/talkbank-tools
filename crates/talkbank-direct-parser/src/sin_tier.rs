//! %sin tier parsing using pure Chumsky combinators.
//!
//! The gesture/sign tier (%sin) provides gesture annotations aligned with main tier words.
//! Format: space-separated tokens, each representing gesture(s) for one word.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Gestures>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Sign_Group>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Sign_Tier>

use crate::whitespace::ws_parser;
use chumsky::prelude::*;
use talkbank_model::ParseOutcome;
use talkbank_model::model::{SinGroupGestures, SinItem, SinTier, SinToken};
use talkbank_model::{
    ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation, Span,
};

/// Parse %sin tier content (without %sin:\t prefix) using chumsky combinators.
///
/// This is the entry point for the ChatParser::parse_sin_tier API.
pub fn parse_sin_tier_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<SinTier> {
    let parser = sin_content_parser();
    match parser.parse(input).into_result() {
        Ok(sin_tier) => ParseOutcome::parsed(sin_tier),
        Err(parse_errors) => {
            for err in parse_errors {
                let span = err.span();
                let msg = format!("Sin tier parse error: {}", err.reason());
                errors.report(ParseError::new(
                    ErrorCode::SinParseError,
                    Severity::Error,
                    SourceLocation::new(Span::from_usize(span.start + offset, span.end + offset)),
                    ErrorContext::new(
                        input,
                        Span::from_usize(span.start + offset, span.end + offset),
                        input,
                    ),
                    msg,
                ));
            }
            ParseOutcome::rejected()
        }
    }
}

/// Parse sin tier content only (items), without the prefix.
fn sin_content_parser<'a>() -> impl Parser<'a, &'a str, SinTier, extra::Err<Rich<'a, char>>> {
    // Parse sin items separated by whitespace
    let items = sin_item_parser()
        .separated_by(ws_parser())
        .at_least(1)
        .collect::<Vec<_>>();

    items.map(SinTier::new)
}

/// Parse a single sin token.
fn sin_token_parser<'a>() -> impl Parser<'a, &'a str, SinItem, extra::Err<Rich<'a, char>>> {
    // Sin tokens can contain almost any characters except whitespace and group delimiters
    // Common formats: "0", "g:ball:dpoint", "g:toy:hold"
    // Grammar: none_of(" \t\n\r〔〕")
    none_of(" \t\n\r\u{3014}\u{3015}") // Exclude space, tab, newline, CR, and 〔〕
        .repeated()
        .at_least(1)
        .to_slice()
        .try_map(|s: &str, span: SimpleSpan| {
            SinToken::new(s)
                .map(SinItem::Token)
                .ok_or_else(|| Rich::custom(span, "Empty sin token"))
        })
}

/// Parse a sin group: 〔 content 〕
///
/// Grammar: sin_begin_group (〔) + sin_grouped_content + sin_end_group (〕)
fn sin_group_parser<'a>() -> impl Parser<'a, &'a str, SinItem, extra::Err<Rich<'a, char>>> {
    just('\u{3014}') // 〔 U+3014 LEFT TORTOISE SHELL BRACKET
        .ignore_then(ws_parser().or_not()) // Optional leading whitespace
        .ignore_then(
            // Parse tokens inside group separated by whitespace
            none_of(" \t\n\r\u{3014}\u{3015}")
                .repeated()
                .at_least(1)
                .to_slice()
                .try_map(|s: &str, span: SimpleSpan| {
                    SinToken::new(s).ok_or_else(|| Rich::custom(span, "Empty sin token in group"))
                })
                .separated_by(ws_parser())
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then_ignore(ws_parser().or_not()) // Optional trailing whitespace
        .then_ignore(just('\u{3015}')) // 〕 U+3015 RIGHT TORTOISE SHELL BRACKET
        .map(|tokens: Vec<SinToken>| SinItem::SinGroup(SinGroupGestures::new(tokens)))
}

/// Parse a single sin item (token or group).
fn sin_item_parser<'a>() -> impl Parser<'a, &'a str, SinItem, extra::Err<Rich<'a, char>>> {
    // Try group first (more specific), then fall back to token
    sin_group_parser().or(sin_token_parser())
}
