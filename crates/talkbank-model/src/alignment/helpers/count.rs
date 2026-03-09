//! Alignment-domain counting/extraction over main-tier content trees.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Phonology_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Sign_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Word_Timing_Tier>

use crate::model::{
    BracketedContent, BracketedItem, ReplacedWord, ScopedAnnotation, UtteranceContent, Word,
};

use super::domain::AlignmentDomain;
use super::rules::{
    annotations_have_alignment_ignore, is_tag_marker_separator,
    should_align_replaced_word_in_pho_sin, should_skip_group, word_is_alignable,
};
use super::to_chat_display_string as to_string;

/// One extracted alignable item shown in mismatch diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct AlignableItem {
    /// Display text for this item (e.g., "hello", "&-um", ".")
    pub text: String,
    /// Optional description for complex items (e.g., "[/]" for retracing)
    pub description: Option<String>,
}

/// Extract alignable items with their display text for a given alignment domain.
///
/// The returned sequence matches alignment traversal order and is used to build
/// human-readable mismatch diagnostics (`main` vs dependent-tier views).
pub fn extract_alignable_items(
    content: &[UtteranceContent],
    domain: AlignmentDomain,
) -> Vec<AlignableItem> {
    let mut items = Vec::new();
    for item in content {
        extract_alignable_from_item(item, domain, &mut items);
    }
    items
}

/// Count alignable units for a given alignment domain.
///
/// This is the fast path for preflight length checks before building richer
/// positional mismatch details.
pub fn count_alignable_content(content: &[UtteranceContent], domain: AlignmentDomain) -> usize {
    content
        .iter()
        .map(|item| count_alignable_item(item, domain))
        .sum()
}

/// Count alignable content up to (but not including) a specific index.
///
/// This is useful for LSP hover features where you need to know how many
/// alignable items precede a given position in the content array.
/// The result uses the same domain-specific inclusion rules as full alignment.
///
/// # Parameters
/// - `content`: The utterance content to count
/// - `max_index`: Only count items before this index (exclusive)
/// - `domain`: The alignment domain (Mor, Pho, or Sin)
///
/// # Returns
/// The count of alignable items in `content[0..max_index]`
///
/// # Examples
/// ```
/// use talkbank_model::alignment::{count_alignable_until, AlignmentDomain};
/// use talkbank_model::model::{UtteranceContent, Word};
///
/// let content = vec![
///     UtteranceContent::Word(Box::new(Word::new_unchecked("hello", "hello"))),
///     UtteranceContent::Word(Box::new(Word::new_unchecked("world", "world"))),
/// ];
///
/// // Count items before index 1 (only first word)
/// let count = count_alignable_until(&content, 1, AlignmentDomain::Mor);
/// assert_eq!(count, 1);
///
/// // Count items before index 2 (both words)
/// let count = count_alignable_until(&content, 2, AlignmentDomain::Mor);
/// assert_eq!(count, 2);
/// ```
pub fn count_alignable_until(
    content: &[UtteranceContent],
    max_index: usize,
    domain: AlignmentDomain,
) -> usize {
    content
        .iter()
        .take(max_index)
        .map(|item| count_alignable_item(item, domain))
        .sum()
}

/// Counts one main-tier item's contribution in the target alignment domain.
fn count_alignable_item(item: &UtteranceContent, domain: AlignmentDomain) -> usize {
    match item {
        UtteranceContent::Word(word) => count_alignable_word(word, &[], domain),
        UtteranceContent::AnnotatedWord(annotated) => {
            count_alignable_word(&annotated.inner, &annotated.scoped_annotations, domain)
        }
        UtteranceContent::ReplacedWord(replaced) => count_alignable_replaced_word(replaced, domain),
        UtteranceContent::Group(group) => count_bracketed_alignable_content(&group.content, domain),
        UtteranceContent::AnnotatedGroup(annotated) => {
            if should_skip_group(&annotated.scoped_annotations, domain) {
                0
            } else {
                count_bracketed_alignable_content(&annotated.inner.content, domain)
            }
        }
        UtteranceContent::PhoGroup(pho) => match domain {
            AlignmentDomain::Mor | AlignmentDomain::Wor => {
                count_bracketed_alignable_content(&pho.content, domain)
            }
            AlignmentDomain::Pho => 1,
            AlignmentDomain::Sin => 0,
        },
        UtteranceContent::SinGroup(sin) => match domain {
            AlignmentDomain::Mor | AlignmentDomain::Wor => {
                count_bracketed_alignable_content(&sin.content, domain)
            }
            AlignmentDomain::Sin => 1,
            AlignmentDomain::Pho => 0,
        },
        UtteranceContent::Quotation(quot) => {
            count_bracketed_alignable_content(&quot.content, domain)
        }
        UtteranceContent::Separator(sep) => {
            if domain == AlignmentDomain::Mor && is_tag_marker_separator(sep) {
                1
            } else {
                0
            }
        }
        UtteranceContent::Pause(_) => {
            // Pauses are phonological events that get transcribed in %pho tiers
            // but NOT in %wor tiers (which only contain actual words)
            // %mor and %sin also don't align to pauses
            if domain == AlignmentDomain::Pho { 1 } else { 0 }
        }
        UtteranceContent::AnnotatedAction(_) => {
            if domain == AlignmentDomain::Sin {
                1
            } else {
                0
            }
        }
        // All remaining variants are non-alignable for every dependent tier:
        // events, markers, formatting, freecodes, overlap points, internal bullets.
        UtteranceContent::Event(_)
        | UtteranceContent::AnnotatedEvent(_)
        | UtteranceContent::Freecode(_)
        | UtteranceContent::OverlapPoint(_)
        | UtteranceContent::InternalBullet(_)
        | UtteranceContent::LongFeatureBegin(_)
        | UtteranceContent::LongFeatureEnd(_)
        | UtteranceContent::UnderlineBegin(_)
        | UtteranceContent::UnderlineEnd(_)
        | UtteranceContent::NonvocalBegin(_)
        | UtteranceContent::NonvocalEnd(_)
        | UtteranceContent::NonvocalSimple(_)
        | UtteranceContent::OtherSpokenEvent(_) => 0,
    }
}

/// Counts bracketed content recursively for alignment in `domain`.
fn count_bracketed_alignable_content(content: &BracketedContent, domain: AlignmentDomain) -> usize {
    content
        .content
        .iter()
        .map(|item| count_bracketed_item(item, domain))
        .sum()
}

/// Counts one bracketed item's alignment contribution in `domain`.
fn count_bracketed_item(item: &BracketedItem, domain: AlignmentDomain) -> usize {
    match item {
        BracketedItem::Word(word) => count_alignable_word(word, &[], domain),
        BracketedItem::AnnotatedWord(annotated) => {
            count_alignable_word(&annotated.inner, &annotated.scoped_annotations, domain)
        }
        BracketedItem::ReplacedWord(replaced) => count_alignable_replaced_word(replaced, domain),
        BracketedItem::AnnotatedGroup(annotated) => {
            if should_skip_group(&annotated.scoped_annotations, domain) {
                0
            } else {
                count_bracketed_alignable_content(&annotated.inner.content, domain)
            }
        }
        BracketedItem::PhoGroup(pho) => match domain {
            AlignmentDomain::Mor | AlignmentDomain::Wor => {
                count_bracketed_alignable_content(&pho.content, domain)
            }
            AlignmentDomain::Pho => 1,
            AlignmentDomain::Sin => 0,
        },
        BracketedItem::SinGroup(sin) => match domain {
            AlignmentDomain::Mor | AlignmentDomain::Wor => {
                count_bracketed_alignable_content(&sin.content, domain)
            }
            AlignmentDomain::Sin => 1,
            AlignmentDomain::Pho => 0,
        },
        BracketedItem::Quotation(quot) => count_bracketed_alignable_content(&quot.content, domain),
        BracketedItem::Separator(sep) => {
            if domain == AlignmentDomain::Mor && is_tag_marker_separator(sep) {
                1
            } else {
                0
            }
        }
        // All remaining variants are non-alignable inside bracketed content:
        // pauses, actions, events, markers, formatting, freecodes, overlap points.
        BracketedItem::Event(_)
        | BracketedItem::AnnotatedEvent(_)
        | BracketedItem::Pause(_)
        | BracketedItem::Action(_)
        | BracketedItem::AnnotatedAction(_)
        | BracketedItem::OverlapPoint(_)
        | BracketedItem::InternalBullet(_)
        | BracketedItem::Freecode(_)
        | BracketedItem::LongFeatureBegin(_)
        | BracketedItem::LongFeatureEnd(_)
        | BracketedItem::UnderlineBegin(_)
        | BracketedItem::UnderlineEnd(_)
        | BracketedItem::NonvocalBegin(_)
        | BracketedItem::NonvocalEnd(_)
        | BracketedItem::NonvocalSimple(_)
        | BracketedItem::OtherSpokenEvent(_) => 0,
    }
}

/// Counts one word token after per-domain exclusion rules.
fn count_alignable_word(
    word: &Word,
    annotations: &[ScopedAnnotation],
    domain: AlignmentDomain,
) -> usize {
    // For individual words, retrace annotations only skip for Mor domain.
    // Retraced words may still appear in %pho/%sin (they were spoken, just corrected).
    // Note: Groups with retrace skip ALL domains - see should_skip_group.
    if domain == AlignmentDomain::Mor && annotations_have_alignment_ignore(annotations) {
        return 0;
    }

    if !word_is_alignable(word, domain) {
        return 0;
    }

    1
}

/// Counts a `ReplacedWord` node after replacement/retrace rules.
fn count_alignable_replaced_word(entry: &ReplacedWord, domain: AlignmentDomain) -> usize {
    // For replaced words (like groups), retrace annotations only skip for Mor domain
    if domain == AlignmentDomain::Mor
        && annotations_have_alignment_ignore(&entry.scoped_annotations)
    {
        return 0;
    }

    match domain {
        AlignmentDomain::Mor | AlignmentDomain::Wor => {
            // %mor and %wor align to replacement words when present.
            // Python batchalign's lexer completely substitutes the replacement text,
            // so both morphological analysis and word-level timing use the corrected form.
            if !entry.replacement.words.is_empty() {
                entry
                    .replacement
                    .words
                    .iter()
                    .filter(|word| word_is_alignable(word, domain))
                    .count()
            } else if word_is_alignable(&entry.word, domain) {
                1
            } else {
                0
            }
        }
        AlignmentDomain::Pho | AlignmentDomain::Sin => {
            // %pho and %sin align to the original word (what was actually
            // spoken/produced), not the replacement. This means a replaced word
            // always contributes at most 1 item, regardless of how many replacement
            // words there are.
            if should_align_replaced_word_in_pho_sin(
                &entry.word,
                !entry.replacement.words.is_empty(),
            ) {
                1
            } else {
                0
            }
        }
    }
}

/// Extracts alignable units from one top-level utterance content item.
///
/// Output order matches traversal order so mismatch diagnostics map cleanly to
/// the original transcript sequence.
fn extract_alignable_from_item(
    item: &UtteranceContent,
    domain: AlignmentDomain,
    output: &mut Vec<AlignableItem>,
) {
    match item {
        UtteranceContent::Word(word) => extract_alignable_from_word(word, &[], domain, output),
        UtteranceContent::AnnotatedWord(annotated) => extract_alignable_from_word(
            &annotated.inner,
            &annotated.scoped_annotations,
            domain,
            output,
        ),
        UtteranceContent::ReplacedWord(replaced) => {
            extract_alignable_from_replaced_word(replaced, domain, output)
        }
        UtteranceContent::Group(group) => {
            extract_alignable_from_bracketed_content(&group.content, domain, output)
        }
        UtteranceContent::AnnotatedGroup(annotated) => {
            if !should_skip_group(&annotated.scoped_annotations, domain) {
                extract_alignable_from_bracketed_content(&annotated.inner.content, domain, output)
            }
        }
        UtteranceContent::PhoGroup(pho) => match domain {
            AlignmentDomain::Mor | AlignmentDomain::Wor => {
                extract_alignable_from_bracketed_content(&pho.content, domain, output)
            }
            AlignmentDomain::Pho => {
                output.push(AlignableItem {
                    text: to_string(pho),
                    description: Some("phonological group".to_string()),
                });
            }
            AlignmentDomain::Sin => {}
        },
        UtteranceContent::SinGroup(sin) => match domain {
            AlignmentDomain::Mor | AlignmentDomain::Wor => {
                extract_alignable_from_bracketed_content(&sin.content, domain, output)
            }
            AlignmentDomain::Sin => {
                output.push(AlignableItem {
                    text: to_string(sin),
                    description: Some("sign group".to_string()),
                });
            }
            AlignmentDomain::Pho => {}
        },
        UtteranceContent::Quotation(quot) => {
            extract_alignable_from_bracketed_content(&quot.content, domain, output)
        }
        UtteranceContent::Separator(sep) => {
            if domain == AlignmentDomain::Mor && is_tag_marker_separator(sep) {
                output.push(AlignableItem {
                    text: to_string(sep),
                    description: None,
                });
            }
        }
        UtteranceContent::Pause(pause) => {
            if domain == AlignmentDomain::Pho {
                output.push(AlignableItem {
                    text: to_string(pause),
                    description: Some("pause".to_string()),
                });
            }
        }
        UtteranceContent::AnnotatedAction(action) => {
            if domain == AlignmentDomain::Sin {
                output.push(AlignableItem {
                    text: to_string(action),
                    description: Some("action".to_string()),
                });
            }
        }
        // All remaining variants produce no alignable items:
        // events, markers, formatting, freecodes, overlap points, internal bullets.
        UtteranceContent::Event(_)
        | UtteranceContent::AnnotatedEvent(_)
        | UtteranceContent::Freecode(_)
        | UtteranceContent::OverlapPoint(_)
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

/// Extracts alignable units from bracketed content recursively.
///
/// Nested groups/quotations are traversed depth-first while preserving document
/// order in the emitted list.
fn extract_alignable_from_bracketed_content(
    content: &BracketedContent,
    domain: AlignmentDomain,
    output: &mut Vec<AlignableItem>,
) {
    for item in &content.content {
        extract_alignable_from_bracketed_item(item, domain, output);
    }
}

/// Extracts alignable units from one bracketed item variant.
///
/// This mirrors top-level extraction rules but for bracket-scoped structures.
fn extract_alignable_from_bracketed_item(
    item: &BracketedItem,
    domain: AlignmentDomain,
    output: &mut Vec<AlignableItem>,
) {
    match item {
        BracketedItem::Word(word) => extract_alignable_from_word(word, &[], domain, output),
        BracketedItem::AnnotatedWord(annotated) => extract_alignable_from_word(
            &annotated.inner,
            &annotated.scoped_annotations,
            domain,
            output,
        ),
        BracketedItem::ReplacedWord(replaced) => {
            extract_alignable_from_replaced_word(replaced, domain, output)
        }
        BracketedItem::AnnotatedGroup(annotated) => {
            if !should_skip_group(&annotated.scoped_annotations, domain) {
                extract_alignable_from_bracketed_content(&annotated.inner.content, domain, output)
            }
        }
        BracketedItem::PhoGroup(pho) => match domain {
            AlignmentDomain::Mor | AlignmentDomain::Wor => {
                extract_alignable_from_bracketed_content(&pho.content, domain, output)
            }
            AlignmentDomain::Pho => {
                output.push(AlignableItem {
                    text: to_string(pho),
                    description: Some("phonological group".to_string()),
                });
            }
            AlignmentDomain::Sin => {}
        },
        BracketedItem::SinGroup(sin) => match domain {
            AlignmentDomain::Mor | AlignmentDomain::Wor => {
                extract_alignable_from_bracketed_content(&sin.content, domain, output)
            }
            AlignmentDomain::Sin => {
                output.push(AlignableItem {
                    text: to_string(sin),
                    description: Some("sign group".to_string()),
                });
            }
            AlignmentDomain::Pho => {}
        },
        BracketedItem::Quotation(quot) => {
            extract_alignable_from_bracketed_content(&quot.content, domain, output)
        }
        BracketedItem::Separator(sep) => {
            if domain == AlignmentDomain::Mor && is_tag_marker_separator(sep) {
                output.push(AlignableItem {
                    text: to_string(sep),
                    description: None,
                });
            }
        }
        // All remaining variants produce no alignable items inside bracketed content:
        // pauses, actions, events, markers, formatting, freecodes, overlap points.
        BracketedItem::Event(_)
        | BracketedItem::AnnotatedEvent(_)
        | BracketedItem::Pause(_)
        | BracketedItem::Action(_)
        | BracketedItem::AnnotatedAction(_)
        | BracketedItem::OverlapPoint(_)
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

/// Extracts one alignable word token after domain-specific filtering.
///
/// For MOR alignment, retrace-ignored annotations suppress the token.
fn extract_alignable_from_word(
    word: &Word,
    annotations: &[ScopedAnnotation],
    domain: AlignmentDomain,
    output: &mut Vec<AlignableItem>,
) {
    if domain == AlignmentDomain::Mor && annotations_have_alignment_ignore(annotations) {
        return;
    }

    if !word_is_alignable(word, domain) {
        return;
    }

    output.push(AlignableItem {
        text: to_string(word),
        description: None,
    });
}

/// Extracts alignable units from a replaced-word node.
///
/// MOR/WOR domains prefer replacement words when available, whereas PHO/SIN
/// keep the originally produced form for alignment.
fn extract_alignable_from_replaced_word(
    entry: &ReplacedWord,
    domain: AlignmentDomain,
    output: &mut Vec<AlignableItem>,
) {
    if domain == AlignmentDomain::Mor
        && annotations_have_alignment_ignore(&entry.scoped_annotations)
    {
        return;
    }

    match domain {
        AlignmentDomain::Mor | AlignmentDomain::Wor => {
            // %mor and %wor align to replacement words when present.
            if !entry.replacement.words.is_empty() {
                for word in &entry.replacement.words {
                    if word_is_alignable(word, domain) {
                        output.push(AlignableItem {
                            text: to_string(word),
                            description: None,
                        });
                    }
                }
            } else if word_is_alignable(&entry.word, domain) {
                output.push(AlignableItem {
                    text: to_string(&entry.word),
                    description: None,
                });
            }
        }
        AlignmentDomain::Pho | AlignmentDomain::Sin => {
            // %pho and %sin align to the original word (what was actually
            // spoken/produced), not the replacement.
            if should_align_replaced_word_in_pho_sin(
                &entry.word,
                !entry.replacement.words.is_empty(),
            ) {
                output.push(AlignableItem {
                    text: to_string(&entry.word),
                    description: None,
                });
            }
        }
    }
}
