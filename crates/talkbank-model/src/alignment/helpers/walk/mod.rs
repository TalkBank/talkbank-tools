//! Closure-based content tree walker for leaf items (words, replaced words, separators).
//!
//! Centralizes the recursive traversal of [`UtteranceContent`] (24 variants) and
//! [`BracketedItem`] (22 variants) so callers provide only leaf-handling logic.
//! Domain-aware group gating (retrace skip for Mor, PhoGroup/SinGroup skip for
//! Pho/Sin) is handled once here.
//!
//! # What the walker handles
//!
//! - Recursion into all 5 group types (Group, AnnotatedGroup, PhoGroup, SinGroup,
//!   Quotation)
//! - `should_skip_group()` gating on AnnotatedGroup when domain is `Some(Mor)`
//! - PhoGroup/SinGroup skipping when domain is `Some(Pho)` or `Some(Sin)`
//!
//! # What callers handle
//!
//! - `word_is_alignable()` filtering
//! - ReplacedWord branch logic (replacement vs original)
//! - Separator filtering (tag-marker check, Mor-only inclusion)
//!
//! # Not suitable for
//!
//! - `strip_timing_from_content()` — also calls `retain()` (container mutation)
//! - `count.rs` — Pho/Sin treat PhoGroup/SinGroup as atomic counted units

use crate::alignment::helpers::{domain::AlignmentDomain, rules::should_skip_group};
use crate::model::{
    BracketedItem, ReplacedWord, ScopedAnnotation, Separator, UtteranceContent, Word,
};

/// Immutable leaf item yielded by [`for_each_leaf`].
pub enum ContentLeaf<'a> {
    /// Plain word or inner word of an `AnnotatedWord`.
    /// The annotation slice is empty for bare words.
    Word(&'a Word, &'a [ScopedAnnotation]),
    /// Replaced word (`word [: replacement]`).
    ReplacedWord(&'a ReplacedWord),
    /// Separator (comma, tag, vocative, etc.).
    Separator(&'a Separator),
}

/// Mutable leaf item yielded by [`for_each_leaf_mut`].
pub enum ContentLeafMut<'a> {
    /// Mutable word reference with shared annotations (split borrow).
    Word(&'a mut Word, &'a [ScopedAnnotation]),
    /// Mutable replaced word reference.
    ReplacedWord(&'a mut ReplacedWord),
    /// Mutable separator reference.
    Separator(&'a mut Separator),
}

/// Walk utterance content and call `f` for each leaf item.
///
/// When `domain` is `Some(Mor)`, annotated groups with retrace/reformulation
/// annotations are skipped. When `domain` is `Some(Pho)` or `Some(Sin)`,
/// PhoGroup and SinGroup are skipped (treated as atomic units by those domains).
/// When `domain` is `None`, all groups are recursed unconditionally.
pub fn for_each_leaf<'a>(
    content: &'a [UtteranceContent],
    domain: Option<AlignmentDomain>,
    f: &mut impl FnMut(ContentLeaf<'a>),
) {
    for item in content {
        match item {
            UtteranceContent::Word(word) => {
                f(ContentLeaf::Word(word, &[]));
            }
            UtteranceContent::AnnotatedWord(annotated) => {
                f(ContentLeaf::Word(
                    &annotated.inner,
                    &annotated.scoped_annotations,
                ));
            }
            UtteranceContent::ReplacedWord(replaced) => {
                f(ContentLeaf::ReplacedWord(replaced));
            }
            UtteranceContent::Separator(sep) => {
                f(ContentLeaf::Separator(sep));
            }
            UtteranceContent::Group(group) => {
                for_each_bracketed_leaf(&group.content.content, domain, f);
            }
            UtteranceContent::AnnotatedGroup(annotated) => {
                if !should_skip_annotated_group(&annotated.scoped_annotations, domain) {
                    for_each_bracketed_leaf(&annotated.inner.content.content, domain, f);
                }
            }
            UtteranceContent::PhoGroup(pho) => {
                if !should_skip_pho_sin_group(domain) {
                    for_each_bracketed_leaf(&pho.content.content, domain, f);
                }
            }
            UtteranceContent::SinGroup(sin) => {
                if !should_skip_pho_sin_group(domain) {
                    for_each_bracketed_leaf(&sin.content.content, domain, f);
                }
            }
            UtteranceContent::Quotation(quot) => {
                for_each_bracketed_leaf(&quot.content.content, domain, f);
            }
            // Non-leaf items: events, pauses, actions, overlap markers, bullets,
            // freecodes, long features, underline markers, nonvocal markers,
            // other spoken events — none produce alignable leaf items.
            UtteranceContent::Event(_)
            | UtteranceContent::AnnotatedEvent(_)
            | UtteranceContent::Pause(_)
            | UtteranceContent::AnnotatedAction(_)
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
}

/// Walk utterance content mutably and call `f` for each leaf item.
///
/// Same domain-aware gating as [`for_each_leaf`].
pub fn for_each_leaf_mut<'a>(
    content: &'a mut [UtteranceContent],
    domain: Option<AlignmentDomain>,
    f: &mut impl FnMut(ContentLeafMut<'a>),
) {
    for item in content {
        match item {
            UtteranceContent::Word(word) => {
                f(ContentLeafMut::Word(word, &[]));
            }
            UtteranceContent::AnnotatedWord(annotated) => {
                // Split borrow: mut inner + shared annotations (disjoint fields).
                let a = annotated.as_mut();
                f(ContentLeafMut::Word(&mut a.inner, &a.scoped_annotations));
            }
            UtteranceContent::ReplacedWord(replaced) => {
                f(ContentLeafMut::ReplacedWord(replaced));
            }
            UtteranceContent::Separator(sep) => {
                f(ContentLeafMut::Separator(sep));
            }
            UtteranceContent::Group(group) => {
                for_each_bracketed_leaf_mut(&mut group.content.content, domain, f);
            }
            UtteranceContent::AnnotatedGroup(annotated) => {
                if !should_skip_annotated_group(&annotated.scoped_annotations, domain) {
                    for_each_bracketed_leaf_mut(&mut annotated.inner.content.content, domain, f);
                }
            }
            UtteranceContent::PhoGroup(pho) => {
                if !should_skip_pho_sin_group(domain) {
                    for_each_bracketed_leaf_mut(&mut pho.content.content, domain, f);
                }
            }
            UtteranceContent::SinGroup(sin) => {
                if !should_skip_pho_sin_group(domain) {
                    for_each_bracketed_leaf_mut(&mut sin.content.content, domain, f);
                }
            }
            UtteranceContent::Quotation(quot) => {
                for_each_bracketed_leaf_mut(&mut quot.content.content, domain, f);
            }
            UtteranceContent::Event(_)
            | UtteranceContent::AnnotatedEvent(_)
            | UtteranceContent::Pause(_)
            | UtteranceContent::AnnotatedAction(_)
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
}

fn for_each_bracketed_leaf<'a>(
    items: &'a [BracketedItem],
    domain: Option<AlignmentDomain>,
    f: &mut impl FnMut(ContentLeaf<'a>),
) {
    for item in items {
        match item {
            BracketedItem::Word(word) => {
                f(ContentLeaf::Word(word, &[]));
            }
            BracketedItem::AnnotatedWord(annotated) => {
                f(ContentLeaf::Word(
                    &annotated.inner,
                    &annotated.scoped_annotations,
                ));
            }
            BracketedItem::ReplacedWord(replaced) => {
                f(ContentLeaf::ReplacedWord(replaced));
            }
            BracketedItem::Separator(sep) => {
                f(ContentLeaf::Separator(sep));
            }
            BracketedItem::AnnotatedGroup(annotated) => {
                if !should_skip_annotated_group(&annotated.scoped_annotations, domain) {
                    for_each_bracketed_leaf(&annotated.inner.content.content, domain, f);
                }
            }
            BracketedItem::PhoGroup(pho) => {
                if !should_skip_pho_sin_group(domain) {
                    for_each_bracketed_leaf(&pho.content.content, domain, f);
                }
            }
            BracketedItem::SinGroup(sin) => {
                if !should_skip_pho_sin_group(domain) {
                    for_each_bracketed_leaf(&sin.content.content, domain, f);
                }
            }
            BracketedItem::Quotation(quot) => {
                for_each_bracketed_leaf(&quot.content.content, domain, f);
            }
            // Non-leaf bracketed items.
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
}

fn for_each_bracketed_leaf_mut<'a>(
    items: &'a mut [BracketedItem],
    domain: Option<AlignmentDomain>,
    f: &mut impl FnMut(ContentLeafMut<'a>),
) {
    for item in items {
        match item {
            BracketedItem::Word(word) => {
                f(ContentLeafMut::Word(word, &[]));
            }
            BracketedItem::AnnotatedWord(annotated) => {
                let a = annotated.as_mut();
                f(ContentLeafMut::Word(&mut a.inner, &a.scoped_annotations));
            }
            BracketedItem::ReplacedWord(replaced) => {
                f(ContentLeafMut::ReplacedWord(replaced));
            }
            BracketedItem::Separator(sep) => {
                f(ContentLeafMut::Separator(sep));
            }
            BracketedItem::AnnotatedGroup(annotated) => {
                if !should_skip_annotated_group(&annotated.scoped_annotations, domain) {
                    for_each_bracketed_leaf_mut(&mut annotated.inner.content.content, domain, f);
                }
            }
            BracketedItem::PhoGroup(pho) => {
                if !should_skip_pho_sin_group(domain) {
                    for_each_bracketed_leaf_mut(&mut pho.content.content, domain, f);
                }
            }
            BracketedItem::SinGroup(sin) => {
                if !should_skip_pho_sin_group(domain) {
                    for_each_bracketed_leaf_mut(&mut sin.content.content, domain, f);
                }
            }
            BracketedItem::Quotation(quot) => {
                for_each_bracketed_leaf_mut(&mut quot.content.content, domain, f);
            }
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
}

/// Returns `true` when an annotated group should be skipped for the given domain.
///
/// Delegates to `should_skip_group()` when a domain is specified.
fn should_skip_annotated_group(
    annotations: &[ScopedAnnotation],
    domain: Option<AlignmentDomain>,
) -> bool {
    match domain {
        Some(d) => should_skip_group(annotations, d),
        None => false,
    }
}

/// Returns `true` when PhoGroup/SinGroup should be skipped.
///
/// Pho and Sin domains treat these as atomic units rather than recursing
/// into their word content.
fn should_skip_pho_sin_group(domain: Option<AlignmentDomain>) -> bool {
    matches!(domain, Some(AlignmentDomain::Pho | AlignmentDomain::Sin))
}

#[cfg(test)]
mod tests;
