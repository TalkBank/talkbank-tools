//! CA (Conversation Analysis) omission normalization for CHAT files.
//!
//! When `@Options` enables CA mode, shortening-only omitted words are converted
//! into explicit CA omission words.

use talkbank_model::model::{
    ChatOptionFlag, Header, Line, MainTier, WordCategory, WordContent, WordShortening, WordText,
};

/// Returns `true` when `@Options` enables CA mode for the file.
pub(crate) fn headers_enable_ca_mode(headers: &[Header]) -> bool {
    headers.iter().any(|header| {
        matches!(header, Header::Options { options } if options.iter().any(ChatOptionFlag::enables_ca_mode))
    })
}

/// Normalize CA omissions in all utterance main tiers.
pub(crate) fn normalize_ca_omissions(lines: &mut [Line]) {
    for line in lines {
        if let Line::Utterance(utterance) = line {
            normalize_ca_omissions_main_tier(&mut utterance.main);
        }
    }
}

/// Normalize omission-marked words in a main tier's utterance content.
pub(crate) fn normalize_ca_omissions_main_tier(main: &mut MainTier) {
    for content in &mut main.content.content {
        normalize_ca_omission_content(content);
    }
}

/// Normalize omission forms inside one utterance-content node.
fn normalize_ca_omission_content(content: &mut talkbank_model::model::UtteranceContent) {
    use talkbank_model::model::UtteranceContent;

    match content {
        UtteranceContent::Word(word) => normalize_ca_omission_word(word),
        UtteranceContent::AnnotatedWord(annotated) => {
            normalize_ca_omission_word(&mut annotated.inner);
        }
        UtteranceContent::ReplacedWord(replaced) => {
            normalize_ca_omission_replaced_word(replaced.as_mut());
        }
        UtteranceContent::Group(group) => normalize_ca_omission_bracketed(&mut group.content),
        UtteranceContent::AnnotatedGroup(annotated) => {
            normalize_ca_omission_bracketed(&mut annotated.inner.content);
        }
        UtteranceContent::PhoGroup(group) => normalize_ca_omission_bracketed(&mut group.content),
        UtteranceContent::SinGroup(group) => normalize_ca_omission_bracketed(&mut group.content),
        UtteranceContent::Quotation(quote) => normalize_ca_omission_bracketed(&mut quote.content),
        UtteranceContent::AnnotatedEvent(_)
        | UtteranceContent::Event(_)
        | UtteranceContent::Pause(_)
        | UtteranceContent::AnnotatedAction(_)
        | UtteranceContent::Freecode(_)
        | UtteranceContent::Separator(_)
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

/// Normalize omission forms inside bracketed content.
fn normalize_ca_omission_bracketed(content: &mut talkbank_model::model::BracketedContent) {
    use talkbank_model::model::BracketedItem;

    for item in &mut content.content {
        match item {
            BracketedItem::Word(word) => normalize_ca_omission_word(word),
            BracketedItem::AnnotatedWord(annotated) => {
                normalize_ca_omission_word(&mut annotated.inner);
            }
            BracketedItem::ReplacedWord(replaced) => {
                normalize_ca_omission_replaced_word(replaced.as_mut());
            }
            BracketedItem::AnnotatedGroup(annotated) => {
                normalize_ca_omission_bracketed(&mut annotated.inner.content);
            }
            BracketedItem::PhoGroup(group) => normalize_ca_omission_bracketed(&mut group.content),
            BracketedItem::SinGroup(group) => normalize_ca_omission_bracketed(&mut group.content),
            BracketedItem::Quotation(quote) => normalize_ca_omission_bracketed(&mut quote.content),
            BracketedItem::Event(_)
            | BracketedItem::AnnotatedEvent(_)
            | BracketedItem::Pause(_)
            | BracketedItem::Action(_)
            | BracketedItem::AnnotatedAction(_)
            | BracketedItem::OverlapPoint(_)
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

/// Normalize omission forms inside replaced-word structures.
fn normalize_ca_omission_replaced_word(replaced: &mut talkbank_model::model::ReplacedWord) {
    normalize_ca_omission_word(&mut replaced.word);
    for word in &mut replaced.replacement.words {
        normalize_ca_omission_word(word);
    }
}

/// Convert shortening-only omitted words into explicit CA omission words.
fn normalize_ca_omission_word(word: &mut talkbank_model::model::Word) {
    if matches!(word.category.as_ref(), Some(category) if *category != WordCategory::CAOmission) {
        return;
    }

    if word.content.len() != 1 {
        return;
    }

    if let WordContent::Shortening(shortening) = word.content[0].clone() {
        let WordShortening(inner) = shortening;
        word.content
            .replace_at(0, WordContent::Text(WordText(inner)));
        word.category = Some(WordCategory::CAOmission);
    }
}
