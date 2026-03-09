//! Linked editing range for speaker codes.
//!
//! When the cursor is on a speaker code, returns all occurrences of that
//! speaker in the document so the editor can highlight them for simultaneous
//! editing. Reuses the same CST walk as the rename feature.

use tower_lsp::lsp_types::*;
use tree_sitter::Tree;

use crate::backend::requests::find_ancestor_kind;
use talkbank_parser::node_types::{ID_SPEAKER, SPEAKER};

/// Find all linked editing ranges for the speaker code under the cursor.
pub fn linked_editing_ranges(
    tree: &Tree,
    doc: &str,
    position: Position,
) -> Option<LinkedEditingRanges> {
    let offset = crate::backend::utils::position_to_offset(doc, position);
    let root = tree.root_node();
    let node = root.descendant_for_byte_range(offset, offset)?;

    // Only activate on speaker nodes.
    let speaker_node =
        find_ancestor_kind(node, SPEAKER).or_else(|| find_ancestor_kind(node, ID_SPEAKER))?;
    let speaker_name = speaker_node.utf8_text(doc.as_bytes()).ok()?.trim();

    if speaker_name.is_empty() {
        return None;
    }

    let mut ranges = Vec::new();
    let index = crate::backend::utils::LineIndex::new(doc);

    collect_speaker_ranges(root, doc, speaker_name, &index, &mut ranges);

    if ranges.is_empty() {
        return None;
    }

    Some(LinkedEditingRanges {
        ranges,
        word_pattern: None,
    })
}

/// Recursively collect ranges of all speaker/id_speaker nodes matching the given name.
fn collect_speaker_ranges(
    node: tree_sitter::Node,
    doc: &str,
    speaker_name: &str,
    index: &crate::backend::utils::LineIndex,
    ranges: &mut Vec<Range>,
) {
    if (node.kind() == SPEAKER || node.kind() == ID_SPEAKER)
        && let Ok(text) = node.utf8_text(doc.as_bytes())
        && text.trim() == speaker_name
    {
        let start = index.offset_to_position(doc, node.start_byte() as u32);
        let end = index.offset_to_position(doc, node.end_byte() as u32);
        ranges.push(Range { start, end });
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_speaker_ranges(child, doc, speaker_name, index, ranges);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_tree(input: &str) -> Tree {
        let mut parser = tree_sitter::Parser::new();
        let language = tree_sitter_talkbank::LANGUAGE;
        parser.set_language(&language.into()).unwrap();
        parser.parse(input, None).unwrap()
    }

    #[test]
    fn test_linked_editing_finds_all_speaker_occurrences() {
        let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child, MOT Mother\n@ID:\teng|corpus|CHI|||||Child|||\n@ID:\teng|corpus|MOT|||||Mother|||\n*CHI:\thello .\n*MOT:\thi .\n*CHI:\tmore .\n@End\n";
        let tree = parse_tree(input);
        // Position on CHI in *CHI: line
        let pos = Position {
            line: 6,
            character: 1,
        };
        let result = linked_editing_ranges(&tree, input, pos);
        assert!(result.is_some());
        let ranges = result.unwrap().ranges;
        // CHI appears in @Participants, @ID, and two *CHI: lines = 4 occurrences
        assert!(
            ranges.len() >= 3,
            "Expected at least 3 CHI occurrences, got {}",
            ranges.len()
        );
    }
}
