//! Overlap validation functions
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#CA_Overlaps>
//! - <https://talkbank.org/0info/manuals/CHAT.html#CA_Delimiters>
//! - <https://talkbank.org/0info/manuals/CHAT.html#QuotationFollows_Linker>
//! - <https://talkbank.org/0info/manuals/CHAT.html#QuotedNewLine_Terminator>

use crate::ErrorSink;
use crate::model::{
    BracketedContent, BracketedItem, OverlapPoint, Utterance, UtteranceContent, Word, WordContent,
};
use crate::validation::{Validate, ValidationContext};

/// Validate overlap-point indices throughout one utterance tree.
///
/// Collects all overlap points from the utterance content and validates their indices.
pub(crate) fn check_overlap_index_values(
    utterance: &Utterance,
    context: &ValidationContext,
    errors: &impl ErrorSink,
) {
    let index_context = context
        .clone()
        .with_field_span(utterance.main.span)
        .with_field_label("overlap_index");
    let mut overlap_points = Vec::new();
    collect_overlap_points(&utterance.main.content.content, &mut overlap_points);

    for point in overlap_points {
        point.validate(&index_context, errors);
    }
}

/// Collect overlap points from top-level utterance content recursively.
fn collect_overlap_points(content: &[UtteranceContent], out: &mut Vec<OverlapPoint>) {
    for item in content {
        match item {
            UtteranceContent::Word(word) => collect_overlap_points_from_word(word, out),
            UtteranceContent::AnnotatedWord(word) => {
                collect_overlap_points_from_word(&word.inner, out);
            }
            UtteranceContent::ReplacedWord(replaced) => {
                collect_overlap_points_from_word(&replaced.word, out);
                for word in &replaced.replacement.words {
                    collect_overlap_points_from_word(word, out);
                }
            }
            UtteranceContent::Group(group) => {
                collect_overlap_points_from_bracketed(&group.content, out);
            }
            UtteranceContent::AnnotatedGroup(group) => {
                collect_overlap_points_from_bracketed(&group.inner.content, out);
            }
            UtteranceContent::PhoGroup(group) => {
                collect_overlap_points_from_bracketed(&group.content, out);
            }
            UtteranceContent::SinGroup(group) => {
                collect_overlap_points_from_bracketed(&group.content, out);
            }
            UtteranceContent::Quotation(quote) => {
                collect_overlap_points_from_bracketed(&quote.content, out);
            }
            UtteranceContent::OverlapPoint(point) => {
                out.push(point.clone());
            }
            UtteranceContent::AnnotatedEvent(_)
            | UtteranceContent::Event(_)
            | UtteranceContent::Pause(_)
            | UtteranceContent::AnnotatedAction(_)
            | UtteranceContent::Freecode(_)
            | UtteranceContent::Separator(_)
            | UtteranceContent::InternalBullet(_)
            | UtteranceContent::LongFeatureBegin(_)
            | UtteranceContent::LongFeatureEnd(_)
            | UtteranceContent::UnderlineBegin(_)
            | UtteranceContent::UnderlineEnd(_)
            | UtteranceContent::NonvocalBegin(_)
            | UtteranceContent::NonvocalEnd(_)
            | UtteranceContent::NonvocalSimple(_)
            | UtteranceContent::OtherSpokenEvent(_) => {}
        }
    }
}

/// Collect overlap points from bracketed content recursively.
fn collect_overlap_points_from_bracketed(content: &BracketedContent, out: &mut Vec<OverlapPoint>) {
    for item in &content.content {
        match item {
            BracketedItem::Word(word) => collect_overlap_points_from_word(word, out),
            BracketedItem::AnnotatedWord(word) => {
                collect_overlap_points_from_word(&word.inner, out);
            }
            BracketedItem::ReplacedWord(replaced) => {
                collect_overlap_points_from_word(&replaced.word, out);
                for word in &replaced.replacement.words {
                    collect_overlap_points_from_word(word, out);
                }
            }
            BracketedItem::AnnotatedGroup(group) => {
                collect_overlap_points_from_bracketed(&group.inner.content, out);
            }
            BracketedItem::PhoGroup(group) => {
                collect_overlap_points_from_bracketed(&group.content, out);
            }
            BracketedItem::SinGroup(group) => {
                collect_overlap_points_from_bracketed(&group.content, out);
            }
            BracketedItem::Quotation(quote) => {
                collect_overlap_points_from_bracketed(&quote.content, out);
            }
            BracketedItem::OverlapPoint(point) => {
                out.push(point.clone());
            }
            BracketedItem::Event(_)
            | BracketedItem::AnnotatedEvent(_)
            | BracketedItem::Pause(_)
            | BracketedItem::Action(_)
            | BracketedItem::AnnotatedAction(_)
            | BracketedItem::Separator(_)
            | BracketedItem::InternalBullet(_)
            | BracketedItem::Freecode(_)
            | BracketedItem::LongFeatureBegin(_)
            | BracketedItem::LongFeatureEnd(_)
            | BracketedItem::UnderlineBegin(_)
            | BracketedItem::UnderlineEnd(_)
            | BracketedItem::NonvocalBegin(_)
            | BracketedItem::NonvocalEnd(_)
            | BracketedItem::NonvocalSimple(_)
            | BracketedItem::OtherSpokenEvent(_) => {}
        }
    }
}

/// Collect overlap points embedded directly inside one word token.
fn collect_overlap_points_from_word(word: &Word, out: &mut Vec<OverlapPoint>) {
    for item in &word.content {
        if let WordContent::OverlapPoint(point) = item {
            out.push(point.clone());
        }
    }
}
