//! Header parsing for CHAT file metadata lines.
//!
//! This module parses both fixed headers (for example `@UTF8`, `@Begin`,
//! `@End`) and value-carrying headers such as `@Languages`, `@Participants`,
//! `@ID`, and `@Media`.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Languages_Header>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Working_with_Media>

mod complex;
mod dispatch;
mod helpers;
mod simple;
mod standalone;

use talkbank_model::model::Header;
use talkbank_model::{ErrorSink, ParseOutcome};

use dispatch::parse_known_header;
use helpers::{
    first_parse_reason, recovered_header_text, report_parse_errors, unknown_header_with_reason,
};

pub use standalone::{parse_id_header_standalone, parse_participant_entry_standalone};

/// Parse a header line using chumsky combinators.
///
/// Handles known header types and falls back to Unknown for others.
pub fn parse_header_impl(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<Header> {
    match parse_known_header(input) {
        Ok(header) => ParseOutcome::parsed(header),
        Err(parse_errors) => {
            let parse_reason = first_parse_reason(parse_errors.first(), "Malformed header syntax");
            report_parse_errors(parse_errors, input, offset, "Header parse error", errors);

            if matches!(input.as_bytes().first(), Some(b'@')) {
                ParseOutcome::parsed(unknown_header_with_reason(
                    recovered_header_text(input),
                    parse_reason,
                    Some("Fix header syntax to match CHAT format"),
                ))
            } else {
                ParseOutcome::rejected()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_header_impl;
    use talkbank_model::ErrorCollector;
    use talkbank_model::ParseOutcome;
    use talkbank_model::model::{BulletContentSegment, Header};

    #[test]
    fn parse_header_comment_with_bullet_segments() {
        let errors = ErrorCollector::new();
        let parsed = parse_header_impl(
            "@Comment:\tThis is timed \u{0015}1234_1567\u{0015}",
            0,
            &errors,
        );

        match parsed {
            ParseOutcome::Parsed(Header::Comment { content }) => {
                assert_eq!(content.segments.len(), 2, "expected text + bullet segments");
                assert!(matches!(
                    &content.segments[0],
                    BulletContentSegment::Text(text) if text.text == "This is timed "
                ));
                assert!(matches!(
                    &content.segments[1],
                    BulletContentSegment::Bullet(timing)
                        if timing.start_ms == 1234 && timing.end_ms == 1567
                ));
            }
            other => panic!("expected parsed bullet-bearing @Comment, got {:?}", other),
        }
    }
}
