//! Bidirectional alignment highlights on click.
//!
//! When the user clicks a word on the main tier, all aligned items across
//! dependent tiers (`%mor`, `%pho`, `%mod`, `%sin`) are highlighted. Clicking
//! on a dependent-tier item highlights the main-tier word *and* corresponding
//! items on sibling tiers. This gives annotators an instant visual check of
//! cross-tier alignment without leaving the source buffer.
//!
//! Tier-specific logic lives in [`tier_handlers`]; CST traversal for computing
//! LSP ranges lives in [`range_finders`].

use tower_lsp::lsp_types::*;

use crate::backend::utils;
use talkbank_model::Span;
use talkbank_model::dependent_tier::DependentTier;

mod range_finders;
mod tier_handlers;

use tier_handlers::{
    highlights_from_gra_tier, highlights_from_main_tier, highlights_from_mod_tier,
    highlights_from_mor_tier, highlights_from_pho_tier, highlights_from_sin_tier,
};

/// Generate document highlights for aligned items across tiers
pub fn document_highlights(
    chat_file: &talkbank_model::model::ChatFile,
    tree: &tree_sitter::Tree,
    position: Position,
    document: &str,
) -> Option<Vec<DocumentHighlight>> {
    // Find the utterance at this position
    let utterance = utils::find_utterance_at_position(chat_file, position, document)?;
    let offset = utils::position_to_offset(document, position) as u32;

    if span_contains(utterance.main.span, offset) {
        // Main tier - find alignment index and highlight across all tiers
        highlights_from_main_tier(utterance, tree, position, document)
    } else {
        let tier = find_dependent_tier_at_offset(utterance, offset)?;
        match tier {
            DependentTier::Mor(_) => highlights_from_mor_tier(utterance, tree, position, document),
            DependentTier::Pho(_) => highlights_from_pho_tier(utterance, tree, position, document),
            DependentTier::Mod(_) => highlights_from_mod_tier(utterance, tree, position, document),
            DependentTier::Sin(_) => highlights_from_sin_tier(utterance, tree, position, document),
            DependentTier::Gra(_) => highlights_from_gra_tier(utterance, tree, position, document),
            _ => None,
        }
    }
}

/// Finds dependent tier at offset.
fn find_dependent_tier_at_offset(
    utterance: &talkbank_model::model::Utterance,
    offset: u32,
) -> Option<&DependentTier> {
    utterance
        .dependent_tiers
        .iter()
        .find(|tier| dependent_tier_span(tier).is_some_and(|span| span_contains(span, offset)))
}

/// Return the source span for a dependent tier variant.
fn dependent_tier_span(tier: &DependentTier) -> Option<Span> {
    Some(tier.span())
}

/// Return `true` when `outer` fully contains `inner`.
fn span_contains(span: Span, offset: u32) -> bool {
    offset >= span.start && offset <= span.end
}
