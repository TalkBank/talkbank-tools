//! Helper parsers for header content, whitespace, and text tokens.
//!
//! These are the building blocks used by both simple and complex header parsers.

use chumsky::prelude::*;
use talkbank_model::model::{Header, WarningText};
use talkbank_model::{ErrorCode, ErrorSink, ParseError, Severity, Span};

// ============================================================================
// Header Content Parser (handles continuation lines)
// ============================================================================

/// Parse header content that may contain continuation markers (\n\t, \r\n\t).
///
/// Converts continuation markers to spaces using pure chumsky combinators.
/// This allows headers to span multiple lines:
/// ```text
/// @Comment:\tLanguage of Caregivers Mother...,
/// \tFather..., Massachusetts
/// ```
/// Parse header content that preserves continuation markers literally (e.g., @Situation:)
pub(super) fn header_whitespace_literal<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    choice((just("\r\n\t"), just("\n\t"), just("\t"), just(" ")))
        .map(|value: &str| value.to_string())
}

/// Parse header whitespace while normalizing continuation markers to spaces.
pub(super) fn header_whitespace_normalized<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    choice((
        just("\r\n\t").to(" ".to_string()),
        just("\n\t").to(" ".to_string()),
        just("\t").to("\t".to_string()),
        just(" ").to(" ".to_string()),
    ))
}

/// Parse one non-whitespace header token.
pub(super) fn header_text_token<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    none_of("\r\n\t ").map(|c: char| c.to_string())
}

/// Concatenate parsed header tokens into a single content string.
fn header_content_from_tokens<'a, P>(
    whitespace: P,
) -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone
where
    P: Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone,
{
    let token = choice((whitespace, header_text_token()));
    token
        .repeated()
        .collect::<Vec<String>>()
        .map(|parts| parts.concat())
}

/// Parse header content and trim leading/trailing whitespace tokens.
fn header_content_trimmed_from_tokens<'a, P>(
    whitespace: P,
) -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone
where
    P: Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone,
{
    let word = header_text_token()
        .repeated()
        .at_least(1)
        .collect::<Vec<String>>()
        .map(|parts| parts.concat());
    let segment = whitespace
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<String>>()
        .map(|parts| parts.concat())
        .then(word.clone())
        .map(|(spaces, token)| format!("{}{}", spaces, token));
    let core = word
        .clone()
        .then(segment.repeated().collect::<Vec<String>>())
        .map(|(first, rest)| {
            let mut output = first;
            for part in rest {
                output.push_str(&part);
            }
            output
        });

    whitespace
        .clone()
        .repeated()
        .ignore_then(core)
        .then_ignore(whitespace.repeated())
}

/// Parse header content while trimming only trailing whitespace tokens.
fn header_content_trimmed_end_from_tokens<'a, P>(
    whitespace: P,
) -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone
where
    P: Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone,
{
    let word = header_text_token()
        .repeated()
        .at_least(1)
        .collect::<Vec<String>>()
        .map(|parts| parts.concat());
    let segment = whitespace
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<String>>()
        .map(|parts| parts.concat())
        .then(word.clone())
        .map(|(spaces, token)| format!("{}{}", spaces, token));
    let core = word
        .clone()
        .then(segment.repeated().collect::<Vec<String>>())
        .map(|(first, rest)| {
            let mut output = first;
            for part in rest {
                output.push_str(&part);
            }
            output
        });
    let leading = whitespace
        .repeated()
        .collect::<Vec<String>>()
        .map(|parts| parts.concat());

    leading.then(core.or_not()).map(|(lead, core)| match core {
        Some(value) => format!("{}{}", lead, value),
        None => lead,
    })
}

/// Parse header content that preserves continuation markers literally.
fn header_content_literal<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    header_content_from_tokens(header_whitespace_literal())
}

/// Parse literal header content and trim outer whitespace.
pub(super) fn header_content_trimmed<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    header_content_trimmed_from_tokens(header_whitespace_literal())
}

/// Parse literal header content and trim trailing whitespace only.
pub(super) fn header_content_trimmed_end<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    header_content_trimmed_end_from_tokens(header_whitespace_literal())
}

/// Parse normalized header content with continuation markers folded to spaces.
pub(super) fn header_content_normalized_trimmed<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    header_content_trimmed_from_tokens(header_whitespace_normalized())
}

/// Default header content parser (preserves continuation markers)
pub(super) fn header_content<'a>()
-> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone {
    header_content_literal()
}

/// Build an `Unknown` header payload with parse diagnostics.
pub(super) fn unknown_header_with_reason(
    text: &str,
    reason: impl Into<String>,
    suggested_fix: Option<&str>,
) -> Header {
    Header::Unknown {
        text: WarningText::new(text.to_string()),
        parse_reason: Some(reason.into()),
        suggested_fix: suggested_fix.map(str::to_string),
    }
}

/// Build a readable parse-recovery reason from the first chumsky error.
pub(super) fn first_parse_reason(parse_error: Option<&Rich<'_, char>>, default: &str) -> String {
    parse_error
        .map(|error| format!("{default}: {}", error.reason()))
        .unwrap_or_else(|| default.to_string())
}

/// Recover header text while trimming only trailing line endings.
pub(super) fn recovered_header_text(input: &str) -> &str {
    let recovered = input.trim_end_matches(['\n', '\r']);
    if recovered.is_empty() {
        input
    } else {
        recovered
    }
}

/// Report chumsky parse errors through the shared header diagnostic shape.
pub(super) fn report_parse_errors<'a>(
    parse_errors: impl IntoIterator<Item = Rich<'a, char>>,
    input: &str,
    offset: usize,
    message_prefix: &str,
    errors: &impl ErrorSink,
) {
    for err in parse_errors {
        let span = err.span();
        let msg = format!("{message_prefix}: {}", err.reason());
        errors.report(ParseError::from_source_span(
            ErrorCode::new("E501"),
            Severity::Error,
            Span::from_usize(span.start + offset, span.end + offset),
            input,
            input,
            msg,
        ));
    }
}
