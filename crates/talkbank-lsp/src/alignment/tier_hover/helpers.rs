//! Shared lookup helpers for tier hover resolvers.
//!
//! Provides CST-based `find_*_item_index_at_offset` helpers, alignment-pair
//! lookup utilities, text-tier offset resolution, and shared `%mor`/`%gra`
//! formatting helpers so each tier-specific resolver can stay focused on its
//! own composition rules.

use crate::backend::utils;
use talkbank_model::Span;
use talkbank_model::alignment::IndexPair;
use talkbank_model::model::{GrammaticalRelation, Mor, MorTier};
use talkbank_parser::node_types::{GRA_RELATION, MOR_CONTENT, PHO_GROUP, SIN_GROUP};
use tower_lsp::lsp_types::Position;
use tree_sitter::{Node, Tree};

/// Extract the primary lemma from a `%mor` item for display.
///
/// Extracts the lemma from the main word.
pub fn extract_mor_stem(mor: &Mor) -> Option<String> {
    Some(mor.main.lemma.to_string())
}

/// Format a `%mor` word/chunk label using its primary lemma when available.
pub fn format_mor_word_label(mor_tier: Option<&MorTier>, word_index: usize) -> String {
    mor_tier
        .and_then(|mor| mor.items.get(word_index.saturating_sub(1)))
        .and_then(extract_mor_stem)
        .unwrap_or_else(|| format!("word {word_index}"))
}

/// Format the `%gra` summary shown alongside aligned `%mor`/main-tier hover info.
pub fn format_gra_alignment_text(mor_tier: &MorTier, relation: &GrammaticalRelation) -> String {
    let head_word = if relation.head == 0 {
        "ROOT".to_string()
    } else {
        format!(
            "{} (word {})",
            format_mor_word_label(Some(mor_tier), relation.head),
            relation.head
        )
    };

    let head_role = if relation.head == 0 {
        "root of sentence"
    } else {
        "dependent"
    };

    format!("{} → {} ({head_role})", relation.relation, head_word)
}

/// Convert an LSP position into a document byte offset.
pub fn position_to_offset(document: &str, position: Position) -> u32 {
    utils::position_to_offset(document, position) as u32
}

/// Find the source-side index aligned to `target_index`.
pub fn find_source_index_for_target<P: IndexPair>(
    pairs: &[P],
    target_index: usize,
) -> Option<usize> {
    pairs
        .iter()
        .find(|pair| pair.target() == Some(target_index))
        .and_then(|pair| pair.source())
}

/// Find the target-side index aligned to `source_index`.
pub fn find_target_index_for_source<P: IndexPair>(
    pairs: &[P],
    source_index: usize,
) -> Option<usize> {
    pairs
        .iter()
        .find(|pair| pair.source() == Some(source_index))
        .and_then(|pair| pair.target())
}

/// Find `%mor` item index at `offset` within `tier_span`.
pub fn find_mor_item_index_at_offset(tree: &Tree, tier_span: Span, offset: u32) -> Option<usize> {
    find_node_index_at_offset(tree, tier_span, MOR_CONTENT, offset)
}

/// Find `%pho` item index at `offset` within `tier_span`.
pub fn find_pho_item_index_at_offset(tree: &Tree, tier_span: Span, offset: u32) -> Option<usize> {
    find_node_index_at_offset(tree, tier_span, PHO_GROUP, offset)
}

/// Find `%sin` item index at `offset` within `tier_span`.
pub fn find_sin_item_index_at_offset(tree: &Tree, tier_span: Span, offset: u32) -> Option<usize> {
    find_node_index_at_offset(tree, tier_span, SIN_GROUP, offset)
}

/// Find `%gra` relation index at `offset` within `tier_span`.
pub fn find_gra_item_index_at_offset(tree: &Tree, tier_span: Span, offset: u32) -> Option<usize> {
    find_node_index_at_offset(tree, tier_span, GRA_RELATION, offset)
}

/// Find which rendered text item contains `offset` inside a text-only tier span.
pub fn find_text_item_index_at_offset<T>(
    span: Span,
    items: &[T],
    offset: u32,
    document: &str,
    item_len: impl Fn(&T) -> usize,
) -> Option<usize> {
    let tier_text = document.get(span.start as usize..span.end as usize)?;
    let content_start = tier_text.find(":\t").map(|p| p + 2)?;
    let content_offset = span.start + u32::try_from(content_start).ok()?;
    let content = &tier_text[content_start..];

    let mut pos = 0usize;
    for (idx, item) in items.iter().enumerate() {
        while pos < content.len() && content.as_bytes().get(pos) == Some(&b' ') {
            pos += 1;
        }

        let rendered_len = item_len(item);
        let item_start = content_offset + u32::try_from(pos).ok()?;
        let item_end = item_start + u32::try_from(rendered_len).ok()?;
        if offset >= item_start && offset <= item_end {
            return Some(idx);
        }

        pos += rendered_len;
    }

    None
}

/// Generic node-index resolver for tier-local items of one tree-sitter kind.
fn find_node_index_at_offset(
    tree: &Tree,
    tier_span: Span,
    kind: &str,
    offset: u32,
) -> Option<usize> {
    let root = tree.root_node();
    let mut stack = vec![root];
    let mut current_idx = 0usize;

    while let Some(node) = stack.pop() {
        if !node_overlaps_span(node, tier_span) {
            continue;
        }

        if !node.is_missing() && node.kind() == kind && node_inside_span(node, tier_span) {
            let start = node.start_byte() as u32;
            let end = node.end_byte() as u32;
            if offset >= start && offset <= end {
                return Some(current_idx);
            }
            current_idx += 1;
        }

        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }

    None
}

/// Return `true` when a tree-sitter node intersects the given span.
fn node_overlaps_span(node: Node, span: Span) -> bool {
    let start = node.start_byte() as u32;
    let end = node.end_byte() as u32;
    end >= span.start && start <= span.end
}

/// Return `true` when a tree-sitter node is fully inside the given span.
fn node_inside_span(node: Node, span: Span) -> bool {
    let start = node.start_byte() as u32;
    let end = node.end_byte() as u32;
    start >= span.start && end <= span.end
}

#[cfg(test)]
mod tests {
    use super::{
        find_source_index_for_target, find_target_index_for_source, find_text_item_index_at_offset,
    };
    use talkbank_model::Span;
    use talkbank_model::alignment::{AlignmentPair, GraAlignmentPair};

    #[test]
    fn source_lookup_matches_target_index_in_pair_not_row_position() {
        let pairs = vec![
            GraAlignmentPair::new(None, Some(0)),
            GraAlignmentPair::new(Some(0), Some(1)),
        ];

        assert_eq!(find_source_index_for_target(&pairs, 1), Some(0));
        assert_eq!(find_source_index_for_target(&pairs, 0), None);
    }

    #[test]
    fn target_lookup_matches_source_index_in_pair_not_row_position() {
        let pairs = vec![
            AlignmentPair::new(None, Some(0)),
            AlignmentPair::new(Some(0), Some(1)),
        ];

        assert_eq!(find_target_index_for_source(&pairs, 0), Some(1));
    }

    #[test]
    fn text_item_lookup_tracks_offsets_inside_tier_content() {
        let document = "%modsyl:\tab cde";
        let span = Span::from_usize(0, document.len());
        let items = ["ab", "cde"];

        assert_eq!(
            find_text_item_index_at_offset(span, &items, 10, document, |item| item.len()),
            Some(0)
        );
        assert_eq!(
            find_text_item_index_at_offset(span, &items, 13, document, |item| item.len()),
            Some(1)
        );
    }
}
