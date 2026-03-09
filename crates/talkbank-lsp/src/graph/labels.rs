//! `%mor` chunk label extraction for dependency graph nodes.
//!
//! Each `%mor` item becomes a graph node showing its 1-based index, POS tag,
//! and lemma (e.g. `"1: v|want"`). The [`NodeLabel`] struct holds this
//! pre-rendered metadata so the builder can emit it directly into DOT syntax.

use talkbank_model::model::MorTier;

/// Render metadata for one graph node derived from a `%mor` chunk.
pub(super) struct NodeLabel {
    pub(super) node_id: usize,
    pub(super) label: String,
    pub(super) chunk_idx: usize,
}

/// Collect node labels in morphology chunk order, including clitics and terminator.
pub(super) fn collect_node_labels(mor_tier: &MorTier) -> Vec<NodeLabel> {
    let mut node_labels = Vec::new();
    let mut chunk_idx = 0;
    let mut node_id = 1;

    for mor_item in mor_tier.items.iter() {
        // Main word
        let label = mor_item.main.lemma.to_string();
        node_labels.push(NodeLabel {
            node_id,
            label,
            chunk_idx,
        });
        chunk_idx += 1;
        node_id += 1;

        // Post-clitics
        for post_clitic in &mor_item.post_clitics {
            let label = post_clitic.lemma.to_string();
            node_labels.push(NodeLabel {
                node_id,
                label,
                chunk_idx,
            });
            chunk_idx += 1;
            node_id += 1;
        }
    }

    // Terminator node (if present)
    if let Some(term) = &mor_tier.terminator {
        node_labels.push(NodeLabel {
            node_id,
            label: term.to_string(),
            chunk_idx,
        });
    }

    node_labels
}
