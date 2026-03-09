//! Small parsing helpers shared by direct-parser modules.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

use talkbank_model::Span;

/// Build a span from byte offsets.
#[inline]
#[allow(dead_code)]
pub fn make_span(start: usize, end: usize) -> Span {
    Span::from_usize(start, end)
}
