//! Handling for unknown/malformed headers during validation.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Comment_Header>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Warning_Header>

use crate::{ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation, Span};

/// Reports unknown or malformed header labels.
///
/// Legacy `@OldHeader` is intentionally downgraded to warning severity so
/// archival corpora can still validate with actionable but non-blocking output.
pub(super) fn check_unknown_header(
    text: &str,
    parse_reason: Option<&str>,
    suggested_fix: Option<&str>,
    span: Span,
    errors: &impl ErrorSink,
) {
    let trimmed = text.trim_start();
    let is_legacy_header = trimmed
        .split_once(':')
        .map(|(label, _)| label.eq_ignore_ascii_case("@OldHeader"))
        .unwrap_or(false);
    let code = if is_legacy_header {
        ErrorCode::LegacyWarning
    } else {
        ErrorCode::UnknownHeader
    };
    let severity = if is_legacy_header {
        Severity::Warning
    } else {
        Severity::Error
    };

    let mut message = if is_legacy_header {
        format!("Legacy header encountered: {}", text)
    } else {
        format!("Unknown or malformed header: {}", text)
    };

    // Append parse reason if available
    if let Some(reason) = parse_reason {
        message.push_str(&format!(" ({})", reason));
    }

    let mut error = ParseError::new(
        code,
        severity,
        SourceLocation::at_offset(span.start as usize),
        ErrorContext::new(text, 0..text.len(), "unknown_header"),
        message,
    );
    error.location.span = span;

    // Use suggested fix if available, otherwise use generic suggestion
    if let Some(fix) = suggested_fix {
        error = error.with_suggestion(fix);
    } else if is_legacy_header {
        error = error.with_suggestion(
            "Keep for archival fidelity, or migrate this metadata into a standard CHAT header",
        );
    } else {
        error = error.with_suggestion(
            "Check the CHAT manual for valid header types: https://talkbank.org/0info/manuals/CHAT.html#File_Headers",
        );
    }

    errors.report(error);
}
