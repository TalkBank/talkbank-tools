use talkbank_model::ParseOutcome;
use talkbank_model::dependent_tier::{DependentTier, UserDefinedDependentTier};
use talkbank_model::model::{NonEmptyString, ParseHealthTier};
use talkbank_model::{ErrorCode, ErrorSink, ParseError, Severity, Span};

/// Emit a standardized invalid-dependent-tier diagnostic.
pub(super) fn report_invalid_dependent_tier(
    input: &str,
    span: Span,
    errors: &impl ErrorSink,
    message: impl Into<String>,
) {
    errors.report(ParseError::from_source_span(
        ErrorCode::InvalidDependentTier,
        Severity::Error,
        span,
        input,
        input,
        message,
    ));
}

/// Create a UserDefined dependent tier from label and content strings.
pub(super) fn make_user_defined_tier(
    input: &str,
    label: &str,
    label_offset: usize,
    content: &str,
    content_offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<DependentTier> {
    let Some(label_ne) = NonEmptyString::new(label) else {
        report_invalid_dependent_tier(
            input,
            Span::from_usize(label_offset, label_offset + label.len()),
            errors,
            "User-defined dependent tier label cannot be empty",
        );
        return ParseOutcome::rejected();
    };

    let Some(content_ne) = NonEmptyString::new(content) else {
        report_invalid_dependent_tier(
            input,
            Span::from_usize(content_offset, content_offset + content.len()),
            errors,
            format!("%{} tier content cannot be empty", label),
        );
        return ParseOutcome::rejected();
    };

    ParseOutcome::parsed(DependentTier::UserDefined(UserDefinedDependentTier {
        label: label_ne,
        content: content_ne,
        span: Span::DUMMY,
    }))
}

/// Create an Unsupported dependent tier from label and content strings.
///
/// Used for tiers that are not recognized CHAT standard tiers and are not
/// `%x`-prefixed user-defined tiers (e.g. `%foo`, `%custom`).
pub(super) fn make_unsupported_tier(
    input: &str,
    label: &str,
    label_offset: usize,
    content: &str,
    content_offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<DependentTier> {
    let Some(label_ne) = NonEmptyString::new(label) else {
        report_invalid_dependent_tier(
            input,
            Span::from_usize(label_offset, label_offset + label.len()),
            errors,
            "Unsupported dependent tier label cannot be empty",
        );
        return ParseOutcome::rejected();
    };

    let Some(content_ne) = NonEmptyString::new(content) else {
        report_invalid_dependent_tier(
            input,
            Span::from_usize(content_offset, content_offset + content.len()),
            errors,
            format!("%{} tier content cannot be empty", label),
        );
        return ParseOutcome::rejected();
    };

    ParseOutcome::parsed(DependentTier::Unsupported(UserDefinedDependentTier {
        label: label_ne,
        content: content_ne,
        span: Span::DUMMY,
    }))
}

/// Parse a text-only dependent tier and reject empty content.
pub(super) fn parse_non_empty_text_tier(
    input: &str,
    label: &str,
    content: &str,
    content_offset: usize,
    errors: &impl ErrorSink,
    wrap: impl FnOnce(talkbank_model::dependent_tier::TextTier) -> DependentTier,
) -> ParseOutcome<DependentTier> {
    let Some(value) = NonEmptyString::new(content) else {
        report_invalid_dependent_tier(
            input,
            Span::from_usize(content_offset, content_offset + content.len()),
            errors,
            format!("%{} tier content cannot be empty", label),
        );
        return ParseOutcome::rejected();
    };

    ParseOutcome::parsed(wrap(talkbank_model::dependent_tier::TextTier::new(value)))
}

/// Split `%label:\tcontent` into (`label`, `content`, `content_offset_delta`).
///
/// Returns label without `%`, content string after `:\t`, and the byte offset from
/// start-of-input to the content start.
pub(super) fn split_tier_label_and_content(input: &str) -> Option<(&str, &str, usize)> {
    let bytes = input.as_bytes();
    if bytes.first().copied() != Some(b'%') {
        return None;
    }

    let sep = bytes.windows(2).position(|w| w == b":\t")?;
    if sep <= 1 {
        return None;
    }

    let label = &input[1..sep];
    let content_start = sep + 2;
    let content = &input[content_start..];
    Some((label, content, content_start))
}

/// Extract the dependent-tier label bytes from a possibly malformed line.
pub(super) fn dependent_tier_label_bytes(line: &str) -> Option<&[u8]> {
    let bytes = line.as_bytes();
    if bytes.first().copied() != Some(b'%') {
        return None;
    }

    let mut end = 1usize;
    while end < bytes.len() {
        match bytes[end] {
            b':' | b'\t' | b' ' | b'\r' | b'\n' => break,
            _ => end += 1,
        }
    }

    if end == 1 {
        return None;
    }

    Some(&bytes[1..end])
}

/// Classify a dependent-tier line into the parse-health tier domain it can taint.
///
/// This accepts both fully well-formed lines (`%mor:\t...`) and malformed lines
/// (`%mor ...`) so recovery paths can still taint the correct alignment domain.
pub(crate) fn classify_dependent_tier_parse_health(line: &str) -> Option<ParseHealthTier> {
    match dependent_tier_label_bytes(line)? {
        b"mor" => Some(ParseHealthTier::Mor),
        b"gra" => Some(ParseHealthTier::Gra),
        b"pho" => Some(ParseHealthTier::Pho),
        b"mod" | b"xmod" => Some(ParseHealthTier::Mod),
        b"wor" => Some(ParseHealthTier::Wor),
        b"sin" => Some(ParseHealthTier::Sin),
        _ => None,
    }
}

/// Convert a ParseOutcome into a TierParseResult (for non-recovering tiers).
pub(super) fn wrap_clean_parse(
    outcome: ParseOutcome<DependentTier>,
) -> crate::dependent_tier::dispatch::TierParseResult {
    match outcome {
        ParseOutcome::Parsed(tier) => crate::dependent_tier::dispatch::TierParseResult::Clean(tier),
        ParseOutcome::Rejected => crate::dependent_tier::dispatch::TierParseResult::Failed(None),
    }
}
