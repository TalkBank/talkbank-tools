use super::diagnostics::build_count_mismatch_error;
use crate::Utterance;
use crate::{ErrorCode, ErrorContext, ParseError, Severity, Span};

pub(super) fn build_mor_tier_from_items(
    utterance: &Utterance,
    items: &[crate::model::Mor],
) -> crate::model::MorTier {
    let mut tier = crate::model::MorTier::new_mor(items.to_vec());
    if let Some(t) = utterance.mor_tier() {
        tier.span = t.span;
        tier.terminator = t.terminator.clone();
    }
    tier
}

pub(super) fn build_phonology_alignment_from_counts(
    main: &crate::model::MainTier,
    item_count: usize,
    tier_span: Span,
    tier_label: &str,
) -> crate::alignment::PhoAlignment {
    let mut alignment = crate::alignment::PhoAlignment::new();

    let main_count = crate::alignment::helpers::count_alignable_content(
        &main.content.content,
        crate::alignment::AlignmentDomain::Pho,
    );

    let min_len = main_count.min(item_count);
    for i in 0..min_len {
        alignment = alignment.with_pair(crate::alignment::AlignmentPair::new(Some(i), Some(i)));
    }

    if main_count > item_count {
        let error = ParseError::new(
            ErrorCode::PhoCountMismatchTooFew,
            Severity::Error,
            main.span.into(),
            ErrorContext::new("", main.span.to_range(), ""),
            format!(
                "Main tier has more alignable content than {} tier: expected {} phonological tokens, found {}",
                tier_label, main_count, item_count
            ),
        )
        .with_suggestion(format!(
            "Add phonological tokens to {} tier to match main tier words",
            tier_label
        ));
        alignment = alignment.with_error(error);
        for i in item_count..main_count {
            alignment = alignment.with_pair(crate::alignment::AlignmentPair::new(Some(i), None));
        }
    } else if item_count > main_count {
        let error = ParseError::new(
            ErrorCode::PhoCountMismatchTooMany,
            Severity::Error,
            tier_span.into(),
            ErrorContext::new("", tier_span.to_range(), ""),
            format!(
                "{} tier is longer than main tier: expected {} phonological tokens, found {}",
                tier_label, main_count, item_count
            ),
        )
        .with_suggestion(format!(
            "Remove extra phonological tokens from {} tier",
            tier_label
        ));
        alignment = alignment.with_error(error);
        for i in main_count..item_count {
            alignment = alignment.with_pair(crate::alignment::AlignmentPair::new(None, Some(i)));
        }
    }

    alignment
}

pub(super) fn build_sin_alignment_from_counts(
    main: &crate::model::MainTier,
    item_count: usize,
    tier_span: Span,
) -> crate::alignment::SinAlignment {
    let mut alignment = crate::alignment::SinAlignment::new();

    let main_count = crate::alignment::helpers::count_alignable_content(
        &main.content.content,
        crate::alignment::AlignmentDomain::Sin,
    );

    let min_len = main_count.min(item_count);
    for i in 0..min_len {
        alignment = alignment.with_pair(crate::alignment::AlignmentPair::new(Some(i), Some(i)));
    }

    if main_count > item_count {
        let error = ParseError::new(
            ErrorCode::SinCountMismatchTooFew,
            Severity::Error,
            main.span.into(),
            ErrorContext::new("", main.span.to_range(), ""),
            format!(
                "Main tier has more alignable content than %sin tier: expected {} gesture/sign tokens, found {}",
                main_count, item_count
            ),
        )
        .with_suggestion("Add gesture/sign tokens to %sin tier to match main tier words");
        alignment = alignment.with_error(error);
        for i in item_count..main_count {
            alignment = alignment.with_pair(crate::alignment::AlignmentPair::new(Some(i), None));
        }
    } else if item_count > main_count {
        let error = ParseError::new(
            ErrorCode::SinCountMismatchTooMany,
            Severity::Error,
            tier_span.into(),
            ErrorContext::new("", tier_span.to_range(), ""),
            format!(
                "%sin tier is longer than main tier: expected {} gesture/sign tokens, found {}",
                main_count, item_count
            ),
        )
        .with_suggestion("Remove extra gesture/sign tokens from %sin tier");
        alignment = alignment.with_error(error);
        for i in main_count..item_count {
            alignment = alignment.with_pair(crate::alignment::AlignmentPair::new(None, Some(i)));
        }
    }

    alignment
}

pub(super) fn build_tier_to_tier_alignment(
    source_count: usize,
    source_span: Span,
    source_label: &str,
    target_count: usize,
    target_span: Span,
    target_label: &str,
    mismatch_code: ErrorCode,
) -> crate::alignment::PhoAlignment {
    let mut alignment = crate::alignment::PhoAlignment::new();

    let min_len = source_count.min(target_count);
    for i in 0..min_len {
        alignment = alignment.with_pair(crate::alignment::AlignmentPair::new(Some(i), Some(i)));
    }

    if source_count != target_count {
        alignment = alignment.with_error(build_count_mismatch_error(
            source_count,
            source_span,
            source_label,
            target_count,
            target_span,
            target_label,
            mismatch_code,
        ));
    }

    alignment
}
