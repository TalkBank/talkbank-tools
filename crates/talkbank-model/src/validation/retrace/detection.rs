//! Retrace marker detection across content structures.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Scoped_Symbols>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Retracing_and_Repetition>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>

use crate::model::{BracketedContent, BracketedItem, MainTier, ScopedAnnotation, UtteranceContent};

/// Returns whether a main tier contains any retrace-style annotations.
///
/// This is the fast-path gate before running more expensive retrace rendering
/// and sequence checks.
pub fn contains_retrace_marker(main_tier: &MainTier) -> bool {
    main_tier
        .content
        .content
        .iter()
        .any(utterance_item_has_retrace)
}

/// Returns whether one utterance-content item carries retrace annotations.
///
/// The check descends into nested bracketed structures as needed.
pub fn utterance_item_has_retrace(item: &UtteranceContent) -> bool {
    match item {
        UtteranceContent::AnnotatedWord(ann) => {
            annotations_have_retrace(ann.scoped_annotations.iter())
        }
        UtteranceContent::AnnotatedEvent(ann) => {
            annotations_have_retrace(ann.scoped_annotations.iter())
        }
        UtteranceContent::AnnotatedAction(ann) => {
            annotations_have_retrace(ann.scoped_annotations.iter())
        }
        UtteranceContent::AnnotatedGroup(ann) => {
            annotations_have_retrace(ann.scoped_annotations.iter())
                || bracketed_content_has_retrace(&ann.inner.content)
        }
        UtteranceContent::Group(group) => bracketed_content_has_retrace(&group.content),
        UtteranceContent::PhoGroup(pho) => bracketed_content_has_retrace(&pho.content),
        UtteranceContent::SinGroup(sin) => bracketed_content_has_retrace(&sin.content),
        UtteranceContent::Quotation(quot) => bracketed_content_has_retrace(&quot.content),
        UtteranceContent::ReplacedWord(rw) => {
            rw.scoped_annotations.iter().any(is_retrace_annotation)
        }
        _ => false,
    }
}

/// Returns whether bracketed content contains retrace annotations.
///
/// This helper is shared by both top-level and nested traversal paths.
pub fn bracketed_content_has_retrace(content: &BracketedContent) -> bool {
    content.content.iter().any(bracketed_item_has_retrace)
}

/// Returns whether one bracketed item contains retrace annotations.
///
/// Group-like variants recurse into inner content so nested retraces are not
/// missed during early detection.
pub fn bracketed_item_has_retrace(item: &BracketedItem) -> bool {
    match item {
        BracketedItem::AnnotatedWord(ann) => {
            annotations_have_retrace(ann.scoped_annotations.iter())
        }
        BracketedItem::AnnotatedEvent(ann) => {
            annotations_have_retrace(ann.scoped_annotations.iter())
        }
        BracketedItem::AnnotatedAction(ann) => {
            annotations_have_retrace(ann.scoped_annotations.iter())
        }
        BracketedItem::AnnotatedGroup(ann) => {
            annotations_have_retrace(ann.scoped_annotations.iter())
                || bracketed_content_has_retrace(&ann.inner.content)
        }
        BracketedItem::PhoGroup(pho) => bracketed_content_has_retrace(&pho.content),
        BracketedItem::SinGroup(sin) => bracketed_content_has_retrace(&sin.content),
        BracketedItem::Quotation(quot) => bracketed_content_has_retrace(&quot.content),
        BracketedItem::ReplacedWord(rw) => rw.scoped_annotations.iter().any(is_retrace_annotation),
        _ => false,
    }
}

/// Returns whether an annotation sequence includes any retrace marker type.
///
/// Callers may pass slices, iterators, or borrowed views without allocation.
pub fn annotations_have_retrace<'a>(
    annotations: impl IntoIterator<Item = &'a ScopedAnnotation>,
) -> bool {
    annotations.into_iter().any(is_retrace_annotation)
}

/// Returns whether a scoped annotation variant is retrace-related.
pub fn is_retrace_annotation(annotation: &ScopedAnnotation) -> bool {
    matches!(
        annotation,
        ScopedAnnotation::PartialRetracing
            | ScopedAnnotation::Retracing
            | ScopedAnnotation::MultipleRetracing
            | ScopedAnnotation::Reformulation
            | ScopedAnnotation::UncertainRetracing
    )
}
