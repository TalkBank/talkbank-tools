//! Validators for metadata header formats (`@Date`, `@Time Duration`, `@Time Start`).
//!
//! This module houses header-field format validators that are easier to keep
//! independent from structural header-order logic in `header/structure`.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Date_Header>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Time_Duration_Header>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Time_Start_Header>

use crate::{ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation, Span};

/// Validate `@Date` in canonical `DD-MMM-YYYY` form.
///
/// The checker reports granular diagnostics per component (day/month/year) so
/// users get actionable fixes instead of a single generic format error.
pub(super) fn check_date_format(date: &str, span: Span, errors: &impl ErrorSink) {
    const VALID_MONTHS: &[&str] = &[
        "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
    ];

    let make_err = |context_label: &str, message: String| {
        let mut err = ParseError::new(
            ErrorCode::InvalidDateFormat,
            Severity::Error,
            SourceLocation::at_offset(span.start as usize),
            ErrorContext::new(date, 0..date.len(), context_label),
            message,
        );
        err.location.span = span;
        err
    };

    let parts: Vec<&str> = date.split('-').collect();

    if parts.len() != 3 {
        errors.report(
            make_err(
                date,
                format!(
                    "Invalid @Date format '{}': expected DD-MMM-YYYY with hyphens",
                    date
                ),
            )
            .with_suggestion(
                "Use format: 01-JAN-2024 (two-digit day, uppercase month, four-digit year)",
            ),
        );
        return;
    }

    let (day_str, month_str, year_str) = (parts[0], parts[1], parts[2]);

    if day_str.len() != 2 {
        errors.report(
            make_err(
                day_str,
                format!(
                    "Invalid @Date day '{}': must be exactly two digits",
                    day_str
                ),
            )
            .with_suggestion("Use two-digit day (e.g., 01, 02, 15)"),
        );
    } else if let Ok(day) = day_str.parse::<u8>() {
        if !(1..=31).contains(&day) {
            errors.report(
                make_err(
                    day_str,
                    format!("Invalid @Date day '{}': must be between 01 and 31", day_str),
                )
                .with_suggestion("Use a valid day between 01 and 31"),
            );
        }
    } else {
        errors.report(
            make_err(
                day_str,
                format!("Invalid @Date day '{}': not a number", day_str),
            )
            .with_suggestion("Day must be a number (01-31)"),
        );
    }

    if !VALID_MONTHS.contains(&month_str) {
        let suggestion = if month_str.len() == 3 {
            let upper = month_str.to_uppercase();
            if VALID_MONTHS.contains(&upper.as_str()) {
                format!("Use uppercase month: {}", upper)
            } else {
                "Valid months: JAN, FEB, MAR, APR, MAY, JUN, JUL, AUG, SEP, OCT, NOV, DEC"
                    .to_string()
            }
        } else {
            "Month must be three-letter uppercase abbreviation (e.g., JAN, FEB, MAR)".to_string()
        };

        errors.report(
            make_err(
                month_str,
                format!(
                    "Invalid @Date month '{}': must be an uppercase three-letter abbreviation",
                    month_str
                ),
            )
            .with_suggestion(suggestion),
        );
    }

    if year_str.len() != 4 {
        errors.report(
            make_err(
                year_str,
                format!(
                    "Invalid @Date year '{}': must be exactly four digits",
                    year_str
                ),
            )
            .with_suggestion("Use four-digit year (e.g., 2024)"),
        );
    } else if year_str.parse::<u16>().is_err() {
        errors.report(
            make_err(
                year_str,
                format!("Invalid @Date year '{}': not a number", year_str),
            )
            .with_suggestion("Year must be a four-digit number"),
        );
    }
}

// ── Time format validators (E540, E541) ───────────────────────────────

/// Check whether a string looks like `HH:MM:SS` or `MM:SS` (colons required, values numeric).
fn is_hms(s: &str) -> bool {
    let parts: Vec<&str> = s.split(':').collect();
    (2..=3).contains(&parts.len())
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// Check whether a string looks like `HH:MM:SS.mmm`, `HH:MM:SS`, `MM:SS.mmm`, or `MM:SS`.
fn is_hms_optional_millis(s: &str) -> bool {
    if let Some((hms, millis)) = s.split_once('.') {
        is_hms(hms) && !millis.is_empty() && millis.chars().all(|c| c.is_ascii_digit())
    } else {
        is_hms(s)
    }
}

/// E540: Validate `@Time Duration` format.
///
/// Expected formats:
/// - `HH:MM:SS` (single duration)
/// - `HH:MM:SS-HH:MM:SS` (range with hyphen)
/// - `HH:MM:SS;HH:MM:SS` (range with semicolon)
/// - Comma-separated combinations of the above
pub(super) fn check_time_duration_format(duration: &str, span: Span, errors: &impl ErrorSink) {
    if duration.is_empty() {
        return;
    }

    // Split by comma for multi-segment durations, then check each segment.
    for segment in duration.split(',') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        // Each segment can be a range (hyphen or semicolon-separated) or a single time.
        let valid = if let Some((left, right)) = segment.split_once('-') {
            is_hms(left) && is_hms(right)
        } else if let Some((left, right)) = segment.split_once(';') {
            is_hms(left) && is_hms(right)
        } else {
            is_hms(segment)
        };
        if !valid {
            let mut err = ParseError::new(
                ErrorCode::InvalidTimeDuration,
                Severity::Warning,
                SourceLocation::at_offset(span.start as usize),
                ErrorContext::new(duration, 0..duration.len(), "time_duration"),
                format!("Invalid @Time Duration format: '{}'", duration),
            )
            .with_suggestion("Expected format: HH:MM:SS or HH:MM:SS-HH:MM:SS");
            err.location.span = span;
            errors.report(err);
            return; // Report once for the whole value.
        }
    }
}

/// E541: Validate `@Time Start` format.
///
/// Expected formats:
/// - `MM:SS` or `HH:MM:SS`
/// - `MM:SS.mmm` or `HH:MM:SS.mmm` (with milliseconds)
pub(super) fn check_time_start_format(start: &str, span: Span, errors: &impl ErrorSink) {
    if start.is_empty() {
        return;
    }

    if !is_hms_optional_millis(start) {
        let mut err = ParseError::new(
            ErrorCode::InvalidTimeStart,
            Severity::Warning,
            SourceLocation::at_offset(span.start as usize),
            ErrorContext::new(start, 0..start.len(), "time_start"),
            format!("Invalid @Time Start format: '{}'", start),
        )
        .with_suggestion("Expected format: MM:SS, HH:MM:SS, or HH:MM:SS.mmm");
        err.location.span = span;
        errors.report(err);
    }
}

#[cfg(test)]
mod time_format_tests {
    use super::*;
    use crate::ErrorCollector;

    #[test]
    fn valid_time_duration_single() {
        let errors = ErrorCollector::new();
        check_time_duration_format("01:23:45", Span::DUMMY, &errors);
        assert!(errors.into_vec().is_empty());
    }

    #[test]
    fn valid_time_duration_range_hyphen() {
        let errors = ErrorCollector::new();
        check_time_duration_format("00:00:00-01:30:00", Span::DUMMY, &errors);
        assert!(errors.into_vec().is_empty());
    }

    #[test]
    fn valid_time_duration_range_semicolon() {
        let errors = ErrorCollector::new();
        check_time_duration_format("00:00:00;01:30:00", Span::DUMMY, &errors);
        assert!(errors.into_vec().is_empty());
    }

    #[test]
    fn invalid_time_duration() {
        let errors = ErrorCollector::new();
        check_time_duration_format("foobar", Span::DUMMY, &errors);
        let errs = errors.into_vec();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code, ErrorCode::InvalidTimeDuration);
    }

    #[test]
    fn valid_time_start_hms() {
        let errors = ErrorCollector::new();
        check_time_start_format("01:23:45", Span::DUMMY, &errors);
        assert!(errors.into_vec().is_empty());
    }

    #[test]
    fn valid_time_start_with_millis() {
        let errors = ErrorCollector::new();
        check_time_start_format("01:23:45.678", Span::DUMMY, &errors);
        assert!(errors.into_vec().is_empty());
    }

    #[test]
    fn valid_time_start_mm_ss() {
        let errors = ErrorCollector::new();
        check_time_start_format("23:45", Span::DUMMY, &errors);
        assert!(errors.into_vec().is_empty());
    }

    #[test]
    fn valid_time_start_mm_ss_with_millis() {
        let errors = ErrorCollector::new();
        check_time_start_format("23:45.678", Span::DUMMY, &errors);
        assert!(errors.into_vec().is_empty());
    }

    #[test]
    fn invalid_time_start() {
        let errors = ErrorCollector::new();
        check_time_start_format("not-a-time", Span::DUMMY, &errors);
        let errs = errors.into_vec();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code, ErrorCode::InvalidTimeStart);
    }
}
