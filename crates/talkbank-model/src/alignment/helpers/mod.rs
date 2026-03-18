//! Shared primitives used by `%mor/%pho/%sin/%wor` alignment passes.
//!
//! These helpers centralize domain policy (what counts as alignable, when to
//! ignore annotations, and how replacement branches expand). Keeping those rules
//! in one place prevents drift between per-tier aligners.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Word_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#GrammaticalRelations_Tier>

mod count;
mod domain;
pub mod overlap;
mod rules;
mod walk;

#[cfg(test)]
mod tests;

pub use count::{
    AlignableItem, count_alignable_content, count_alignable_until, extract_alignable_items,
};

/// Render any [`WriteChat`](crate::model::WriteChat) value into owned text
/// for alignment diagnostic messages.
///
/// Best-effort: write failures are silently ignored because diagnostic
/// formatting must never panic the alignment path.
pub fn to_chat_display_string<T: crate::model::WriteChat>(item: &T) -> String {
    let mut s = String::new();
    item.write_chat(&mut s).ok();
    s
}
pub use domain::AlignmentDomain;
pub use overlap::{
    OverlapMarkerInfo, OverlapPointVisit, OverlapRegion, OverlapRegionKind, extract_overlap_info,
    for_each_overlap_point,
};
pub use rules::should_align_replaced_word_in_pho_sin;
pub use rules::{
    annotations_have_alignment_ignore, is_tag_marker_separator, should_skip_group,
    word_is_alignable,
};
pub use walk::{ContentLeaf, ContentLeafMut, for_each_leaf, for_each_leaf_mut};
