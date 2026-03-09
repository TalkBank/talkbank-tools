//! Parses repeated `base_annotations` lists into model annotations.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Scoped_Symbols>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Retracing_and_Repetition>

use crate::error::{ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation};
use crate::model::ScopedAnnotation;
use crate::node_types::WHITESPACES;
use crate::parser::tree_parsing::parser_helpers::is_base_annotation;
use talkbank_model::ParseOutcome;
use tree_sitter::Node;

use super::single::parse_single_annotation;

/// Converts a `base_annotations` node into `Vec<ScopedAnnotation>`.
///
/// **Grammar Rule:**
/// ```text
/// base_annotations: $ => repeat1(
///   seq($.whitespaces, $.base_annotation)
/// )
/// ```
///
/// **Expected Sequential Order:**
/// One or more pairs of: `whitespaces` then `base_annotation`
pub(crate) fn parse_scoped_annotations(
    node: Node,
    source: &str,
    errors: &impl ErrorSink,
) -> Vec<ScopedAnnotation> {
    let child_count = node.child_count();
    // Pre-allocate: child_count / 2 pairs of (whitespace, annotation)
    let mut annotations = Vec::with_capacity(child_count / 2);
    let mut idx = 0;

    // Grammar: repeat1(seq(whitespaces, base_annotation))
    // Expect alternating whitespaces and base_annotation
    while idx < child_count {
        // Expect whitespaces
        if let Some(child) = node.child(idx as u32) {
            if child.kind() == WHITESPACES {
                idx += 1;
            } else {
                errors.report(ParseError::new(
                    ErrorCode::ScopedAnnotationParseError,
                    Severity::Error,
                    SourceLocation::from_offsets(child.start_byte(), child.end_byte()),
                    ErrorContext::new(source, child.start_byte()..child.end_byte(), ""),
                    format!(
                        "Expected 'whitespaces' at position {} of base_annotations, found '{}'",
                        idx,
                        child.kind()
                    ),
                ));
                idx += 1;
                continue;
            }
        } else {
            break;
        }

        // Expect base_annotation (or one of its concrete subtypes)
        if idx < child_count
            && let Some(child) = node.child(idx as u32)
        {
            if is_base_annotation(child.kind()) {
                if let ParseOutcome::Parsed(ann) = parse_single_annotation(child, source, errors) {
                    annotations.push(ann);
                }
                idx += 1;
            } else {
                errors.report(ParseError::new(
                    ErrorCode::ScopedAnnotationParseError,
                    Severity::Error,
                    SourceLocation::from_offsets(child.start_byte(), child.end_byte()),
                    ErrorContext::new(source, child.start_byte()..child.end_byte(), ""),
                    format!(
                        "Expected annotation at position {} of base_annotations, found '{}'",
                        idx,
                        child.kind()
                    ),
                ));
                idx += 1;
            }
        }
    }

    annotations
}
