//! Bracketed content types for groups, quotations, and other nested structures.
//!
//! This module defines content that can appear inside bracketed constructs like
//! regular groups `<...>`, phonological groups `‹...›`, sign groups `〔...〕`,
//! and quotations `"..."`. Bracketed content has the same structure as main tier
//! content but appears within delimiters.
//!
//! # Bracketed Construct Types
//!
//! - **Group** (`<...>`) - Can have scoped annotations, used for retracing
//! - **PhoGroup** (`‹...›`) - Phonological groups, cannot have annotations
//! - **SinGroup** (`〔...〕`) - Sign/gesture groups, cannot have annotations
//! - **Quotation** (`"..."`) - Direct quotes, cannot have annotations
//!
//! # CHAT Format Examples
//!
//! ```text
//! <I want> [/] I need                      Group with retracing
//! <the dog> [//] the cat                   Group with correction
//! ‹hello there›                            Phonological group
//! 〔wave goodbye〕                          Sign group
//! "I said hello"                           Quotation
//! <I want &=pause it>                      Group with nested event
//! ```
//!
//! # References
//!
//! - [Group](https://talkbank.org/0info/manuals/CHAT.html#Group)
//! - [Scoped Symbols](https://talkbank.org/0info/manuals/CHAT.html#Scoped_Symbols)

use crate::model::{
    LongFeatureBegin, LongFeatureEnd, NonvocalBegin, NonvocalEnd, NonvocalSimple, OtherSpokenEvent,
    Separator, UnderlineMarker, Word, WriteChat,
};
use super::{Event, Action, Annotated, ReplacedWord, Pause, OverlapPoint};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::ops::{Deref, DerefMut};

/// Content items that can appear inside bracketed constructs.
///
/// Bracketed items are similar to main tier content but appear within delimiters
/// like `<...>`, `‹...›`, `〔...〕`, or `"..."`. The same word, event, pause, and
/// group types are supported.
///
/// # CHAT Format Examples
///
/// ```text
/// <I want cookie>                          Words in group
/// <I want [* m] it>                        AnnotatedWord in group
/// <the &=laughs dog>                       Event in group
/// <um (.) yeah>                            Pause in group
/// <I <really> want it>                     Nested groups
/// ‹hello there›                            Phonological group content
/// 〔wave goodbye〕                          Sign group content
/// "she said hello"                         Quotation content
/// ```
///
/// # References
///
/// - [Group](https://talkbank.org/0info/manuals/CHAT.html#Group)
/// - [Scoped Symbols](https://talkbank.org/0info/manuals/CHAT.html#Scoped_Symbols)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BracketedItem {
    /// Bare word with NO brackets
    Word(Word),
    /// Word WITH scoped annotations (e.g., "hello [* m]", "hello [= greeting]")
    #[serde(rename = "annotated_word")]
    AnnotatedWord(Annotated<Word>),
    /// Replaced word - word with [: replacement] (e.g., "hello [: world]")
    #[serde(rename = "replaced_word")]
    ReplacedWord(ReplacedWord),
    /// Event (e.g., &=laughs)
    Event(Event),
    /// Event WITH scoped annotations
    #[serde(rename = "annotated_event")]
    AnnotatedEvent(Annotated<Event>),
    /// Pause (e.g., (.), (1.5))
    Pause(Pause),
    /// Action (e.g., 0) - bare action without annotations (rare)
    Action(Action),
    /// Action WITH scoped annotations (e.g., <0 [= ! whining]>)
    #[serde(rename = "annotated_action")]
    AnnotatedAction(Annotated<Action>),
    /// Nested regular group <...> WITH scoped annotations (e.g., retrace: <word> [/])
    /// Note: Groups inside bracketed content MUST have annotations in CHAT format
    #[serde(rename = "annotated_group")]
    AnnotatedGroup(Annotated<super::Group>),
    /// Nested phonological group ‹...›
    #[serde(rename = "pho_group")]
    PhoGroup(super::PhoGroup),
    /// Nested sign group 〔...〕
    #[serde(rename = "sin_group")]
    SinGroup(super::SinGroup),
    /// Nested quotation "..."
    Quotation(super::Quotation),
    /// Overlap point marker (CA transcription: ⌊, ⌋, ⌈, ⌉)
    #[serde(rename = "overlap_point")]
    OverlapPoint(OverlapPoint),
    /// Separator (commas, etc. used in CA transcription)
    /// Canonicalized output always has a space before the separator
    Separator(Separator),
    /// Internal bullet/timestamp within group
    #[serde(rename = "internal_bullet")]
    InternalBullet(super::Bullet),
    /// Freecode - free-form inline annotation (e.g., "[^ comment]")
    Freecode(super::Freecode),
    /// Long feature begin marker (&{l=LABEL)
    #[serde(rename = "long_feature_begin")]
    LongFeatureBegin(LongFeatureBegin),
    /// Long feature end marker (&}l=LABEL)
    #[serde(rename = "long_feature_end")]
    LongFeatureEnd(LongFeatureEnd),
    /// Underline begin marker (\u0002\u0001)
    #[serde(rename = "underline_begin")]
    UnderlineBegin(UnderlineMarker),
    /// Underline end marker (\u0002\u0002)
    #[serde(rename = "underline_end")]
    UnderlineEnd(UnderlineMarker),
    /// Nonvocal scope begin marker (&{n=LABEL)
    #[serde(rename = "nonvocal_begin")]
    NonvocalBegin(NonvocalBegin),
    /// Nonvocal scope end marker (&}n=LABEL)
    #[serde(rename = "nonvocal_end")]
    NonvocalEnd(NonvocalEnd),
    /// Nonvocal simple marker (&{n=LABEL})
    #[serde(rename = "nonvocal_simple")]
    NonvocalSimple(NonvocalSimple),
    /// Other spoken event (&*SPEAKER:word) - someone else speaking in background
    #[serde(rename = "other_spoken_event")]
    OtherSpokenEvent(OtherSpokenEvent),
}

impl WriteChat for BracketedItem {
    /// Serializes one bracket-internal item without surrounding delimiters.
    fn write_chat<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        match self {
            BracketedItem::Word(word) => word.write_chat(w),
            BracketedItem::AnnotatedWord(ann) => ann.write_chat(w),
            BracketedItem::ReplacedWord(rw) => rw.write_chat(w),
            BracketedItem::Event(event) => event.write_chat(w),
            BracketedItem::AnnotatedEvent(ann) => ann.write_chat(w),
            BracketedItem::Pause(pause) => pause.write_chat(w),
            BracketedItem::Action(action) => action.write_chat(w),
            BracketedItem::AnnotatedAction(ann) => ann.write_chat(w),
            BracketedItem::AnnotatedGroup(ann) => ann.write_chat(w),
            BracketedItem::PhoGroup(pho) => pho.write_chat(w),
            BracketedItem::SinGroup(sin) => sin.write_chat(w),
            BracketedItem::Quotation(quot) => quot.write_chat(w),
            BracketedItem::OverlapPoint(marker) => marker.write_chat(w),
            BracketedItem::Separator(sep) => sep.write_chat(w),
            BracketedItem::InternalBullet(bullet) => {
                w.write_char(' ')?;
                bullet.write_chat(w)
            }
            BracketedItem::Freecode(freecode) => freecode.write_chat(w),
            BracketedItem::LongFeatureBegin(begin) => begin.write_chat(w),
            BracketedItem::LongFeatureEnd(end) => end.write_chat(w),
            BracketedItem::UnderlineBegin(_) => {
                w.write_char('\u{0002}')?;
                w.write_char('\u{0001}')
            }
            BracketedItem::UnderlineEnd(_) => {
                w.write_char('\u{0002}')?;
                w.write_char('\u{0002}')
            }
            BracketedItem::NonvocalBegin(begin) => begin.write_chat(w),
            BracketedItem::NonvocalEnd(end) => end.write_chat(w),
            BracketedItem::NonvocalSimple(simple) => simple.write_chat(w),
            BracketedItem::OtherSpokenEvent(event) => event.write_chat(w),
        }
    }
}

/// Container for content inside bracketed constructs.
///
/// Used by all bracketed types: `Group`, `PhoGroup`, `SinGroup`, and `Quotation`.
/// Provides a unified structure for managing sequences of items within delimiters.
///
/// # CHAT Format Examples
///
/// ```text
/// <I want cookie>                          Group content
/// <I want [* m] it>                        Group with annotated word
/// <the &=laughs dog>                       Group with event
/// ‹hello there›                            Phonological group content
/// 〔wave goodbye〕                          Sign group content
/// "she said hello"                         Quotation content
/// ```
///
/// # References
///
/// - [Group](https://talkbank.org/0info/manuals/CHAT.html#Group)
/// - [Scoped Symbols](https://talkbank.org/0info/manuals/CHAT.html#Scoped_Symbols)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BracketedContent {
    /// Content items (words, pauses, events, nested groups, etc.)
    pub content: BracketedItems,
}

impl BracketedContent {
    /// Constructs bracket payload content in transcript order.
    pub fn new(content: Vec<BracketedItem>) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Returns `true` when the bracket payload is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Number of serialized tokens/items inside the bracket.
    pub fn len(&self) -> usize {
        self.content.len()
    }
}

impl WriteChat for BracketedContent {
    /// Serializes bracket payload content only (caller writes delimiters).
    fn write_chat<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        for (i, item) in self.content.iter().enumerate() {
            if i > 0 {
                w.write_char(' ')?;
            }
            item.write_chat(w)?;
        }
        Ok(())
    }
}

/// Ordered list of items inside a bracketed construct.
///
/// Wraps a `Vec<BracketedItem>` and provides collection-like access via `Deref`.
///
/// References:
/// - <https://talkbank.org/0info/manuals/CHAT.html#Group_Scopes>
/// - <https://talkbank.org/0info/manuals/CHAT.html#Scoped_Symbols>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct BracketedItems(pub Vec<BracketedItem>);

impl BracketedItems {
    /// Wraps bracket items while preserving their original order.
    pub fn new(items: Vec<BracketedItem>) -> Self {
        Self(items)
    }

    /// Returns `true` when there are no bracket items.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Deref for BracketedItems {
    type Target = Vec<BracketedItem>;

    /// Borrows the underlying ordered item list.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BracketedItems {
    /// Mutably borrows the underlying ordered item list.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<BracketedItem>> for BracketedItems {
    /// Wraps a raw vector without copying.
    fn from(items: Vec<BracketedItem>) -> Self {
        Self(items)
    }
}

impl<'a> IntoIterator for &'a BracketedItems {
    type Item = &'a BracketedItem;
    type IntoIter = std::slice::Iter<'a, BracketedItem>;

    /// Iterates immutably over bracketed items.
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut BracketedItems {
    type Item = &'a mut BracketedItem;
    type IntoIter = std::slice::IterMut<'a, BracketedItem>;

    /// Iterates mutably over bracketed items.
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl IntoIterator for BracketedItems {
    type Item = BracketedItem;
    type IntoIter = std::vec::IntoIter<BracketedItem>;

    /// Consumes the wrapper and yields owned bracketed items.
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl crate::validation::Validate for BracketedItems {
    /// Bracketed-item semantics are validated in the surrounding annotation context.
    fn validate(
        &self,
        _context: &crate::validation::ValidationContext,
        _errors: &impl crate::ErrorSink,
    ) {
    }
}
