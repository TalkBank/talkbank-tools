//! Parses one scoped annotation token (or wrapper) at a time.
//!
//! This module is the main dispatch point from coarsened tree-sitter nodes
//! into typed `ScopedAnnotation` values.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Scoped_Symbols>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Retracing_and_Repetition>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Comment_Scope>

use crate::error::{ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation};
use crate::model::ScopedAnnotation;
use crate::node_types::{
    ALT_ANNOTATION, BASE_ANNOTATION, DURATION_ANNOTATION, ERROR_MARKER_ANNOTATION, EXCLUDE_MARKER,
    EXPLANATION_ANNOTATION, INDEXED_OVERLAP_FOLLOWS, INDEXED_OVERLAP_PRECEDES, PARA_ANNOTATION,
    PERCENT_ANNOTATION, RETRACE_COMPLETE, RETRACE_MULTIPLE, RETRACE_PARTIAL, RETRACE_REFORMULATION,
    RETRACE_UNCERTAIN, SCOPED_BEST_GUESS, SCOPED_CONTRASTIVE_STRESSING, SCOPED_STRESSING,
    SCOPED_UNCERTAIN,
};
use crate::tokens;
use talkbank_model::ParseOutcome;
use tree_sitter::Node;

/// Converts one annotation node like `[*]`, `[= text]`, or `[<2]`.
///
/// After Phase 5 coarsening, most annotations are atomic tokens. Parsing is
/// delegated to [`crate::tokens`] which provides the canonical
/// parse functions for each coarsened token type.
pub(crate) fn parse_single_annotation(
    node: Node,
    source: &str,
    errors: &impl ErrorSink,
) -> ParseOutcome<ScopedAnnotation> {
    let node_kind = node.kind();

    // With supertypes, we may receive either:
    // 1. A `base_annotation` wrapper node (look at child(0) for the concrete type)
    // 2. A concrete annotation type directly (process node itself)
    let annotation_node = if node_kind == BASE_ANNOTATION {
        let child_count = node.child_count();
        if child_count == 0 {
            errors.report(ParseError::new(
                ErrorCode::ScopedAnnotationParseError,
                Severity::Error,
                SourceLocation::from_offsets(node.start_byte(), node.end_byte()),
                ErrorContext::new(source, node.start_byte()..node.end_byte(), ""),
                "base_annotation has no children (expected exactly one annotation type)",
            ));
            return ParseOutcome::rejected();
        }
        match node.child(0u32) {
            Some(child) => child,
            None => return ParseOutcome::rejected(),
        }
    } else {
        node
    };

    let raw = &source[annotation_node.start_byte()..annotation_node.end_byte()];

    match annotation_node.kind() {
        // Text-bearing annotations — delegate to tokens API
        EXPLANATION_ANNOTATION => delegate_or_error(
            tokens::parse_explanation_token(raw),
            annotation_node,
            source,
            errors,
        ),
        PARA_ANNOTATION => delegate_or_error(
            tokens::parse_para_token(raw),
            annotation_node,
            source,
            errors,
        ),
        ALT_ANNOTATION => delegate_or_error(
            tokens::parse_alt_token(raw),
            annotation_node,
            source,
            errors,
        ),
        PERCENT_ANNOTATION => delegate_or_error(
            tokens::parse_percent_token(raw),
            annotation_node,
            source,
            errors,
        ),
        DURATION_ANNOTATION => delegate_or_error(
            tokens::parse_duration_token(raw),
            annotation_node,
            source,
            errors,
        ),
        ERROR_MARKER_ANNOTATION => delegate_or_error(
            tokens::parse_error_marker_token(raw),
            annotation_node,
            source,
            errors,
        ),
        // Overlap annotations — delegate to tokens API
        INDEXED_OVERLAP_PRECEDES => delegate_or_error(
            tokens::parse_overlap_precedes_token(raw),
            annotation_node,
            source,
            errors,
        ),
        INDEXED_OVERLAP_FOLLOWS => delegate_or_error(
            tokens::parse_overlap_follows_token(raw),
            annotation_node,
            source,
            errors,
        ),
        // Scoped symbols — already atomic tokens, no payload
        SCOPED_STRESSING => ParseOutcome::parsed(ScopedAnnotation::ScopedStressing),
        SCOPED_CONTRASTIVE_STRESSING => {
            ParseOutcome::parsed(ScopedAnnotation::ScopedContrastiveStressing)
        }
        SCOPED_BEST_GUESS => ParseOutcome::parsed(ScopedAnnotation::ScopedBestGuess),
        SCOPED_UNCERTAIN => ParseOutcome::parsed(ScopedAnnotation::ScopedUncertain),
        // Retrace markers — already atomic tokens, no payload
        RETRACE_COMPLETE => ParseOutcome::parsed(ScopedAnnotation::Retracing),
        RETRACE_PARTIAL => ParseOutcome::parsed(ScopedAnnotation::PartialRetracing),
        RETRACE_MULTIPLE => ParseOutcome::parsed(ScopedAnnotation::MultipleRetracing),
        RETRACE_REFORMULATION => ParseOutcome::parsed(ScopedAnnotation::Reformulation),
        RETRACE_UNCERTAIN => ParseOutcome::parsed(ScopedAnnotation::UncertainRetracing),
        // Exclude marker — already atomic token
        EXCLUDE_MARKER => ParseOutcome::parsed(ScopedAnnotation::ExcludeMarker),
        _ => {
            errors.report(ParseError::new(
                ErrorCode::ScopedAnnotationParseError,
                Severity::Error,
                SourceLocation::from_offsets(
                    annotation_node.start_byte(),
                    annotation_node.end_byte(),
                ),
                ErrorContext::new(
                    source,
                    annotation_node.start_byte()..annotation_node.end_byte(),
                    annotation_node.kind(),
                ),
                format!("Unknown base annotation type '{}'", annotation_node.kind()),
            ));
            ParseOutcome::rejected()
        }
    }
}

/// Convert a token parse result into a ParseOutcome, reporting an error on failure.
fn delegate_or_error(
    result: Option<ScopedAnnotation>,
    node: Node,
    source: &str,
    errors: &impl ErrorSink,
) -> ParseOutcome<ScopedAnnotation> {
    match result {
        Some(annotation) => ParseOutcome::parsed(annotation),
        None => {
            errors.report(ParseError::new(
                ErrorCode::ScopedAnnotationParseError,
                Severity::Error,
                SourceLocation::from_offsets(node.start_byte(), node.end_byte()),
                ErrorContext::new(source, node.start_byte()..node.end_byte(), ""),
                format!(
                    "Failed to parse {} token: '{}'",
                    node.kind(),
                    &source[node.start_byte()..node.end_byte()]
                ),
            ));
            ParseOutcome::rejected()
        }
    }
}
