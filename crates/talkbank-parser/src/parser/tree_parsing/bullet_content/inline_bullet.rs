//! Parser for inline bullet tokens (`\u{15}start_end\u{15}`) in tier text.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Working_with_Media>

use crate::error::{ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation};
use crate::parser::tree_parsing::media_bullet::parse_bullet_text;
use talkbank_model::ParseOutcome;
use tree_sitter::Node;

/// Converts one `inline_bullet` node to `(start_ms, end_ms)`.
///
/// After grammar coarsening, `inline_bullet` is a single token matching
/// `\u0015\d+_\d+\u0015`. We extract the node text and parse it
/// with `parse_bullet_text()`.
///
/// **Returns:** ParseOutcome<(u64, u64)> (errors streamed via ErrorSink)
/// - Parsed((start_ms, end_ms)) if valid
/// - Rejected if parsing failed
pub(super) fn parse_inline_bullet(
    node: Node,
    source: &str,
    errors: &impl ErrorSink,
) -> ParseOutcome<(u64, u64)> {
    let text = match node.utf8_text(source.as_bytes()) {
        Ok(t) => t,
        Err(err) => {
            errors.report(ParseError::new(
                ErrorCode::InvalidMediaBullet,
                Severity::Error,
                SourceLocation::from_offsets(node.start_byte(), node.end_byte()),
                ErrorContext::new(source, node.start_byte()..node.end_byte(), ""),
                format!("UTF-8 decoding error in inline bullet: {err}"),
            ));
            return ParseOutcome::rejected();
        }
    };

    let Some((start_ms, end_ms, _skip)) = parse_bullet_text(text) else {
        errors.report(ParseError::new(
            ErrorCode::InvalidMediaBullet,
            Severity::Error,
            SourceLocation::from_offsets(node.start_byte(), node.end_byte()),
            ErrorContext::new(source, node.start_byte()..node.end_byte(), text),
            format!("Invalid inline bullet: could not parse timestamps from '{text}'"),
        ));
        return ParseOutcome::rejected();
    };

    if start_ms == 0 && end_ms == 0 {
        errors.report(ParseError::new(
            ErrorCode::InvalidMediaBullet,
            Severity::Error,
            SourceLocation::from_offsets(node.start_byte(), node.end_byte()),
            ErrorContext::new(source, node.start_byte()..node.end_byte(), ""),
            "Invalid inline bullet: both start and end timestamps are 0",
        ));
        return ParseOutcome::rejected();
    }

    ParseOutcome::parsed((start_ms, end_ms))
}
