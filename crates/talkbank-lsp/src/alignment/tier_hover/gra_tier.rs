//! `%gra` hover resolver — shows the grammatical relation with dependency context.
//!
//! Resolves the `%gra` relation under the cursor (e.g. `1|2|SUBJ`), then looks
//! up the aligned `%mor` item and main-tier word to provide a rich hover card
//! showing the full dependency triple alongside the word and its morphology.

use super::helpers::{
    find_gra_item_index_at_offset, find_source_index_for_target, format_mor_word_label,
    position_to_offset,
};
use crate::alignment::finders::get_alignable_content_by_index;
use crate::alignment::formatters::{format_content_item, format_mor_item};
use crate::alignment::types::AlignmentHoverInfo;
use talkbank_model::model::Utterance;
use tower_lsp::lsp_types::Position;

/// Build hover info for a `%gra` relation under the cursor.
pub fn find_gra_tier_hover_info(
    utterance: &Utterance,
    tree: &tree_sitter::Tree,
    position: Position,
    document: &str,
    _is_translation: bool,
) -> Option<AlignmentHoverInfo> {
    let gra_tier = utterance.gra_tier()?;
    let gra_alignment = utterance
        .alignments
        .as_ref()
        .and_then(|alignments| alignments.gra.as_ref());

    let offset = position_to_offset(document, position);
    let gra_idx = find_gra_item_index_at_offset(tree, gra_tier.span, offset)?;

    // Get the gra relation
    let gra_relation = gra_tier.relations.get(gra_idx)?;

    // Get mor tier to resolve word stems
    let mor_tier = utterance.mor_tier();

    // Find this word's stem from mor tier
    let word_stem = format_mor_word_label(mor_tier, gra_relation.index);

    // Find head word's stem from mor tier
    let head_stem = if gra_relation.head == 0 {
        "ROOT".to_string()
    } else {
        format_mor_word_label(mor_tier, gra_relation.head)
    };

    // Find all children (words that depend on this word)
    let children: Vec<String> = gra_tier
        .relations
        .iter()
        .filter(|r| r.head == gra_relation.index)
        .map(|r| {
            let child_stem = format_mor_word_label(mor_tier, r.index);
            format!("{} ({})", child_stem, r.relation)
        })
        .collect();

    // Build hover info
    let mut details = vec![
        ("Word".to_string(), word_stem),
        ("Relation".to_string(), gra_relation.relation.to_string()),
        ("Head".to_string(), head_stem),
    ];

    // Add children if any
    if !children.is_empty() {
        details.push(("Dependents".to_string(), children.join(", ")));
    }

    let mut info = AlignmentHoverInfo::new(
        "Grammatical Relation",
        format!(
            "{}|{}|{}",
            gra_relation.index, gra_relation.head, gra_relation.relation
        ),
    )
    .with_details(details);

    // Look up aligned %mor item and main tier word via AlignmentSet.
    if let Some(gra_alignment) = gra_alignment
        && let Some(mor_idx) = find_source_index_for_target(&gra_alignment.pairs, gra_idx)
        && let Some(mor_tier) = utterance.mor_tier()
        && let Some(mor_item) = mor_tier.items.get(mor_idx)
    {
        info.aligned_to_mor = Some(format_mor_item(mor_item));

        if let Some(alignments) = utterance.alignments.as_ref() {
            let mor_alignment = alignments.mor.as_ref();
            if let Some(mor_alignment) = mor_alignment
                && let Some(main_idx) = find_source_index_for_target(&mor_alignment.pairs, mor_idx)
                && let Some(main_content) =
                    get_alignable_content_by_index(&utterance.main.content.content, main_idx)
            {
                info.aligned_to_main = Some(format_content_item(main_content));
            }
        }
    }

    Some(info)
}
