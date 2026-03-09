//! Parsing for inline media bullets embedded in main-tier content.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Working_with_Media>

use crate::error::{
    ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation, Span,
};
use crate::model::UtteranceContent;
use crate::parser::tree_parsing::media_bullet::parse_bullet_text;
use talkbank_model::ParseOutcome;
use tree_sitter::Node;

/// Converts an `internal bullet` token into `UtteranceContent::InternalBullet`.
///
/// Internal bullets match the `media_url` token defined in `grammar.js` (`\u0015\d+_\d+-?\u0015`).
/// We parse the encoded timestamps via `parse_bullet_text()` and ensure the resulting range is
/// non-zero so the object can later be correlated with the media tracking described in the CHAT
/// manual’s Main Tier and Working with Media sections.
pub(crate) fn parse_internal_bullet(
    node: Node,
    source: &str,
    errors: &impl ErrorSink,
) -> ParseOutcome<UtteranceContent> {
    let text = match node.utf8_text(source.as_bytes()) {
        Ok(t) => t,
        Err(err) => {
            errors.report(ParseError::new(
                ErrorCode::InvalidMediaBullet,
                Severity::Error,
                SourceLocation::from_offsets(node.start_byte(), node.end_byte()),
                ErrorContext::new(source, node.start_byte()..node.end_byte(), ""),
                format!("UTF-8 decoding error in internal bullet: {err}"),
            ));
            return ParseOutcome::rejected();
        }
    };

    let Some((start_ms, end_ms, skip)) = parse_bullet_text(text) else {
        errors.report(ParseError::new(
            ErrorCode::InvalidMediaBullet,
            Severity::Error,
            SourceLocation::from_offsets(node.start_byte(), node.end_byte()),
            ErrorContext::new(source, node.start_byte()..node.end_byte(), text),
            format!("Invalid internal bullet: could not parse timestamps from '{text}'"),
        ));
        return ParseOutcome::rejected();
    };

    if start_ms == 0 && end_ms == 0 {
        errors.report(ParseError::new(
            ErrorCode::InvalidMediaBullet,
            Severity::Error,
            SourceLocation::from_offsets(node.start_byte(), node.end_byte()),
            ErrorContext::new(source, node.start_byte()..node.end_byte(), ""),
            "Invalid internal bullet: both start and end timestamps are 0",
        ));
        return ParseOutcome::rejected();
    }

    let span = Span::new(node.start_byte() as u32, node.end_byte() as u32);
    let bullet = crate::model::Bullet::new(start_ms, end_ms)
        .with_skip(skip)
        .with_span(span);
    ParseOutcome::parsed(UtteranceContent::InternalBullet(bullet))
}
