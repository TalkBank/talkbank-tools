//! Unit tests for the closure-based content walker.

use super::*;
use crate::Span;
use crate::annotation::AnnotatedScopedAnnotations;
use crate::model::{
    Annotated, BracketedContent, BracketedItem, Group, PhoGroup, ScopedAnnotation, Separator,
    UtteranceContent, Word,
};

fn word(text: &str) -> Word {
    Word::simple(text)
}

fn boxed_word(text: &str) -> Box<Word> {
    Box::new(word(text))
}

/// Collects leaf word texts from content using the walker.
fn collect_word_texts(
    content: &[UtteranceContent],
    domain: Option<AlignmentDomain>,
) -> Vec<String> {
    let mut texts = Vec::new();
    for_each_leaf(content, domain, &mut |leaf| {
        if let ContentLeaf::Word(w, _) = leaf {
            texts.push(w.cleaned_text().to_string());
        }
    });
    texts
}

#[test]
fn flat_words() {
    let content = vec![
        UtteranceContent::Word(boxed_word("hello")),
        UtteranceContent::Word(boxed_word("world")),
    ];
    assert_eq!(collect_word_texts(&content, None), ["hello", "world"]);
}

#[test]
fn words_inside_group() {
    let group = Group::new(BracketedContent::new(vec![
        BracketedItem::Word(boxed_word("in")),
        BracketedItem::Word(boxed_word("group")),
    ]));
    let content = vec![UtteranceContent::Group(group)];
    assert_eq!(collect_word_texts(&content, None), ["in", "group"]);
}

#[test]
fn annotated_group_retrace_skipped_for_mor() {
    let group = Group::new(BracketedContent::new(vec![BracketedItem::Word(
        boxed_word("retraced"),
    )]));
    let annotated = Annotated {
        inner: group,
        scoped_annotations: AnnotatedScopedAnnotations::new(vec![ScopedAnnotation::Retracing]),
        span: Span::DUMMY,
    };
    let content = vec![
        UtteranceContent::AnnotatedGroup(annotated),
        UtteranceContent::Word(boxed_word("kept")),
    ];

    // Mor domain: retrace is skipped
    assert_eq!(
        collect_word_texts(&content, Some(AlignmentDomain::Mor)),
        ["kept"]
    );
    // No domain: retrace is included
    assert_eq!(collect_word_texts(&content, None), ["retraced", "kept"]);
    // Wor domain: retrace is included
    assert_eq!(
        collect_word_texts(&content, Some(AlignmentDomain::Wor)),
        ["retraced", "kept"]
    );
}

#[test]
fn pho_group_skipped_for_pho_domain() {
    let pho = PhoGroup::new(BracketedContent::new(vec![BracketedItem::Word(
        boxed_word("phonological"),
    )]));
    let content = vec![
        UtteranceContent::PhoGroup(pho),
        UtteranceContent::Word(boxed_word("after")),
    ];

    // Pho domain: PhoGroup skipped
    assert_eq!(
        collect_word_texts(&content, Some(AlignmentDomain::Pho)),
        ["after"]
    );
    // Mor domain: PhoGroup recursed
    assert_eq!(
        collect_word_texts(&content, Some(AlignmentDomain::Mor)),
        ["phonological", "after"]
    );
    // No domain: PhoGroup recursed
    assert_eq!(
        collect_word_texts(&content, None),
        ["phonological", "after"]
    );
}

#[test]
fn separator_yielded() {
    let content = vec![
        UtteranceContent::Word(boxed_word("hello")),
        UtteranceContent::Separator(Separator::Comma { span: Span::DUMMY }),
        UtteranceContent::Word(boxed_word("world")),
    ];
    let mut count = 0;
    for_each_leaf(&content, None, &mut |leaf| {
        if let ContentLeaf::Separator(_) = leaf {
            count += 1;
        }
    });
    assert_eq!(count, 1);
}

#[test]
fn mut_walker_modifies_words() {
    let mut content = vec![
        UtteranceContent::Word(boxed_word("hello")),
        UtteranceContent::Word(boxed_word("world")),
    ];
    for_each_leaf_mut(&mut content, None, &mut |leaf| {
        if let ContentLeafMut::Word(w, _) = leaf {
            w.inline_bullet = Some(crate::model::Bullet::new(0, 100));
        }
    });
    // Verify modification took effect
    let mut count = 0;
    for_each_leaf(&content, None, &mut |leaf| {
        if let ContentLeaf::Word(w, _) = leaf {
            assert!(w.inline_bullet.is_some());
            count += 1;
        }
    });
    assert_eq!(count, 2);
}

#[test]
fn nested_quotation_recursion() {
    let quot = crate::model::Quotation::new(BracketedContent::new(vec![BracketedItem::Word(
        boxed_word("quoted"),
    )]));
    let content = vec![UtteranceContent::Quotation(quot)];
    assert_eq!(collect_word_texts(&content, None), ["quoted"]);
}
