//! File-level `ERROR` analysis and fallback diagnostic routing.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

use crate::error::{ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation};
use crate::parser::tree_parsing::parser_helpers::extract_utf8_text;
use tree_sitter::Node;

/// Classifies a top-level `ERROR` node into a specific parse error.
pub(crate) fn analyze_error_node(node: Node, source: &str, errors: &impl ErrorSink) {
    let error_text = extract_utf8_text(node, source, errors, "file_error", "");
    let start = node.start_byte();
    let end = node.end_byte();

    // Check if this is a dependent tier error (starts with %)
    if matches!(error_text.chars().next(), Some('%')) {
        // E710: Invalid %gra - non-numeric index
        if error_text.contains("%gra:") {
            errors.report(
                ParseError::new(
                    ErrorCode::UnexpectedGrammarNode,
                    Severity::Error,
                    SourceLocation::from_offsets(start, end),
                    ErrorContext::new(source, start..end, error_text),
                    "Invalid GRA relation - non-numeric index",
                )
                .with_suggestion(
                    "GRA relation indices must be numbers (e.g., 1|2|SUBJ, not one|2|SUBJ)",
                ),
            );
            return;
        }

        // Recoverable dependent-tier parse failures:
        // keep file parsing alive and let downstream validation report semantic issues.
        let (code, message) = if error_text.contains(":\t") {
            (
                ErrorCode::InvalidDependentTier,
                format!(
                    "Could not fully parse dependent tier: {}",
                    match error_text.lines().next() {
                        Some(line) => line,
                        None => error_text,
                    }
                ),
            )
        } else {
            (
                ErrorCode::MalformedTierHeader,
                format!(
                    "Malformed dependent tier header: {}",
                    match error_text.lines().next() {
                        Some(line) => line,
                        None => error_text,
                    }
                ),
            )
        };

        errors.report(
            ParseError::new(
                code,
                Severity::Warning,
                SourceLocation::from_offsets(start, end),
                ErrorContext::new(source, start..end, error_text),
                message,
            )
            .with_suggestion(
                "Check dependent tier syntax (%tier:\\tcontent) and tier-specific format",
            ),
        );
        return;
    }

    // Check if this is a main tier error (starts with *)
    if matches!(error_text.chars().next(), Some('*')) {
        // E301: Check for empty speaker (*: with no code between * and :)
        if error_text.contains("*:") || error_text.contains("*\t") {
            errors.report(
                ParseError::new(
                    ErrorCode::MissingMainTier,
                    Severity::Error,
                    SourceLocation::from_offsets(start, end),
                    ErrorContext::new(source, start..end, error_text),
                    "Empty speaker code in main tier",
                )
                .with_suggestion("Add a speaker code between * and : (e.g., *CHI:)"),
            );
            return;
        }

        // E305: Check for missing content after speaker (*SPEAKER: with nothing after)
        // Check if error text contains colon and ends with colon (with possible whitespace)
        if let Some(last_colon) = error_text.rfind(':') {
            let trailing_ws = error_text
                .bytes()
                .rev()
                .take_while(|&b| b == b'\n' || b == b'\r' || b == b'\t' || b == b' ')
                .count();
            if trailing_ws + 1 >= error_text.len() - last_colon {
                errors.report(
                    ParseError::new(
                        ErrorCode::MissingTerminator,
                        Severity::Error,
                        SourceLocation::from_offsets(start, end),
                        ErrorContext::new(source, start..end, error_text),
                        "Main tier missing content after speaker",
                    )
                    .with_suggestion(
                        "Add utterance content after the colon-tab (e.g., *CHI:\thello world .)",
                    ),
                );
                return;
            }
        }
    }

    // Check if this is a header error by looking at the content
    if matches!(error_text.chars().next(), Some('@')) {
        // Check for empty headers (missing content after colon)
        // Check if text is just the header name with colon (with only whitespace after)
        if error_text.len() >= 11 && &error_text[..11] == "@Languages:" {
            let after_colon = &error_text[11..];
            if after_colon
                .bytes()
                .all(|b| b == b'\t' || b == b' ' || b == b'\n' || b == b'\r')
            {
                errors.report(ParseError::new(
                    ErrorCode::EmptyLanguagesHeader,
                    Severity::Error,
                    SourceLocation::from_offsets(start, end),
                    ErrorContext::new(source, start..end, error_text),
                    "@Languages header cannot be empty",
                ));
                return;
            }
        } else if error_text.len() >= 6 && &error_text[..6] == "@Date:" {
            let after_colon = &error_text[6..];
            if after_colon
                .bytes()
                .all(|b| b == b'\t' || b == b' ' || b == b'\n' || b == b'\r')
            {
                errors.report(ParseError::new(
                    ErrorCode::EmptyDateHeader,
                    Severity::Error,
                    SourceLocation::from_offsets(start, end),
                    ErrorContext::new(source, start..end, error_text),
                    "@Date header cannot be empty",
                ));
                return;
            }
        } else if error_text.len() >= 7 && &error_text[..7] == "@Media:" {
            let after_colon = &error_text[7..];
            if after_colon
                .bytes()
                .all(|b| b == b'\t' || b == b' ' || b == b'\n' || b == b'\r')
            {
                errors.report(ParseError::new(
                    ErrorCode::EmptyMediaHeader,
                    Severity::Error,
                    SourceLocation::from_offsets(start, end),
                    ErrorContext::new(source, start..end, error_text),
                    "@Media header cannot be empty",
                ));
                return;
            }
        }

        // Check for @ID errors
        // ERROR node with @ID means tree-sitter failed to parse the structure
        // Don't try to manually parse it - just report it's malformed
        if error_text.contains("@ID:") {
            errors.report(
                ParseError::new(
                    ErrorCode::InvalidIDFormat,
                    Severity::Error,
                    SourceLocation::from_offsets(start, end),
                    ErrorContext::new(source, start..end, error_text),
                    "Malformed @ID header - tree-sitter failed to parse structure",
                )
                .with_suggestion(
                    "Format: @ID:\tlang|corpus|speaker|age|sex|group|SES|role|education|custom|",
                ),
            );
            return;
        }
    }

    // Generic file-level error
    errors.report(ParseError::new(
        ErrorCode::UnparsableContent,
        Severity::Error,
        SourceLocation::from_offsets(start, end),
        ErrorContext::new(source, start..end, error_text),
        format!(
            "Could not parse content: {}",
            match error_text.lines().next() {
                Some(line) => line,
                None => error_text,
            }
        ),
    ));
}
