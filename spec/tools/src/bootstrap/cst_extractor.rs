//! Parses a CHAT fixture with tree-sitter and extracts its CST structure,
//! replacing a target node with a `{fragment}` placeholder.
//!
//! The resulting [`CstIr`] tree captures the full CST surrounding the target
//! node so that scaffold templates can wrap arbitrary node fragments in a
//! structurally correct context.

use thiserror::Error;
use tree_sitter::{Node, Parser};

use super::cst_ir::CstIr;

/// Errors that can occur during CST extraction.
#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("Failed to parse CHAT file with tree-sitter")]
    ParseFailed,

    #[error("Node '{0}' not found in CST")]
    NodeNotFound(String),
}

/// Extract CST structure from a CHAT fixture with placeholder at target node
///
/// This is the main public API that combines parsing, finding, and extraction.
///
/// # Arguments
/// * `source` - CHAT file content
/// * `target_kind` - The tree-sitter node kind to extract (e.g., "standalone_word")
///
/// # Returns
/// * `Ok(CstIr)` - CST structure with {fragment} placeholder at the target node
/// * `Err(ExtractError)` - If parsing fails or node is not found
pub fn extract_cst_from_fixture(source: &str, target_kind: &str) -> Result<CstIr, ExtractError> {
    // Parse with tree-sitter
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_talkbank::LANGUAGE.into())
        .map_err(|_| ExtractError::ParseFailed)?;

    let tree = parser
        .parse(source, None)
        .ok_or(ExtractError::ParseFailed)?;
    let root = tree.root_node();

    // Find target node
    let target_node = find_node_by_kind(root, target_kind)
        .ok_or_else(|| ExtractError::NodeNotFound(target_kind.to_string()))?;

    // Extract CST structure with placeholder
    Ok(extract_cst_structure(root, target_node))
}

/// Find the first node of a given kind in the CST using depth-first search
fn find_node_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    // Check if current node matches
    if node.kind() == kind {
        return Some(node);
    }

    // Recursively search children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_node_by_kind(child, kind) {
            return Some(found);
        }
    }

    None
}

/// Extract CST structure with placeholder at target node position
pub fn extract_cst_structure<'a>(node: Node<'a>, target_node: Node<'a>) -> CstIr {
    // Check if current node is the target (structural identity)
    if node.id() == target_node.id() {
        return CstIr::Placeholder("{fragment}".to_string());
    }

    // Otherwise, recursively build Node with children
    let start_pos = node.start_position();
    let end_pos = node.end_position();

    let mut cursor = node.walk();
    let children: Vec<CstIr> = node
        .children(&mut cursor)
        .map(|child| extract_cst_structure(child, target_node))
        .collect();

    CstIr::Node {
        kind: node.kind().to_string(),
        start: (start_pos.row, start_pos.column),
        end: (end_pos.row, end_pos.column),
        children,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use talkbank_parser::node_types;
    use tree_sitter_talkbank::LANGUAGE;

    /// Runs setup parser.
    fn setup_parser() -> Result<Parser> {
        let mut parser = Parser::new();
        parser
            .set_language(&LANGUAGE.into())
            .map_err(|_| ExtractError::ParseFailed)?;
        Ok(parser)
    }

    /// Tests find standalone word.
    #[test]
    fn test_find_standalone_word() -> Result<()> {
        let mut parser = setup_parser()?;
        let input = "@UTF8\n@Begin\n*CHI:\thello .\n@End";
        let tree = parser.parse(input, None).ok_or(ExtractError::ParseFailed)?;
        let root = tree.root_node();

        let word_node = find_node_by_kind(root, node_types::STANDALONE_WORD);
        assert!(word_node.is_some(), "Should find standalone_word node");
        assert_eq!(
            word_node.map(|node| node.kind()),
            Some(node_types::STANDALONE_WORD)
        );
        Ok(())
    }

    /// Tests find main tier.
    #[test]
    fn test_find_main_tier() -> Result<()> {
        let mut parser = setup_parser()?;
        let input = "@UTF8\n@Begin\n*CHI:\thello .\n@End";
        let tree = parser.parse(input, None).ok_or(ExtractError::ParseFailed)?;
        let root = tree.root_node();

        let main_tier = find_node_by_kind(root, node_types::MAIN_TIER);
        assert!(main_tier.is_some(), "Should find main_tier node");
        assert_eq!(
            main_tier.map(|node| node.kind()),
            Some(node_types::MAIN_TIER)
        );
        Ok(())
    }

    /// Tests node not found.
    #[test]
    fn test_node_not_found() -> Result<()> {
        let mut parser = setup_parser()?;
        let input = "@UTF8\n@Begin\n*CHI:\thello .\n@End";
        let tree = parser.parse(input, None).ok_or(ExtractError::ParseFailed)?;
        let root = tree.root_node();

        let result = find_node_by_kind(root, node_types::OVERLAP_POINT);
        assert!(result.is_none(), "Should not find nonexistent node");
        Ok(())
    }

    /// Tests extract with placeholder.
    #[test]
    fn test_extract_with_placeholder() -> Result<()> {
        let mut parser = setup_parser()?;
        let input = "@UTF8\n@Begin\n*CHI:\thello .\n@End";
        let tree = parser.parse(input, None).ok_or(ExtractError::ParseFailed)?;
        let root = tree.root_node();

        let word_node = find_node_by_kind(root, node_types::STANDALONE_WORD)
            .ok_or_else(|| ExtractError::NodeNotFound(node_types::STANDALONE_WORD.to_string()))?;

        let cst = extract_cst_structure(root, word_node);

        // Verify placeholder is injected
        let cst_string = cst.to_string();
        assert!(
            cst_string.contains("{fragment}"),
            "CST should contain placeholder"
        );
        Ok(())
    }

    /// Tests placeholder position.
    #[test]
    fn test_placeholder_position() -> Result<()> {
        let mut parser = setup_parser()?;
        let input = "@UTF8\n@Begin\n*CHI:\thello .\n@End";
        let tree = parser.parse(input, None).ok_or(ExtractError::ParseFailed)?;
        let root = tree.root_node();

        let word_node = find_node_by_kind(root, node_types::STANDALONE_WORD)
            .ok_or_else(|| ExtractError::NodeNotFound(node_types::STANDALONE_WORD.to_string()))?;

        let cst = extract_cst_structure(root, word_node);

        // Helper function to check if placeholder is in structure
        /// Returns whether placeholder.
        fn has_placeholder(cst: &CstIr) -> bool {
            match cst {
                CstIr::Placeholder(_) => true,
                CstIr::Node { children, .. } => children.iter().any(has_placeholder),
            }
        }

        assert!(
            has_placeholder(&cst),
            "Placeholder should be in CST structure"
        );
        Ok(())
    }

    /// Tests extract cst from fixture.
    #[test]
    fn test_extract_cst_from_fixture() -> Result<()> {
        let input = "@UTF8\n@Begin\n*CHI:\thello .\n@End";

        // Test successful extraction
        let result = extract_cst_from_fixture(input, node_types::STANDALONE_WORD);
        assert!(result.is_ok(), "Should extract CST successfully");

        let cst = result?;
        let cst_string = cst.to_string();
        assert!(
            cst_string.contains("{fragment}"),
            "CST should contain placeholder"
        );
        assert!(
            cst_string.contains("(document"),
            "CST should have document root"
        );

        // Test node not found
        let result = extract_cst_from_fixture(input, node_types::OVERLAP_POINT);
        assert!(result.is_err(), "Should fail when node not found");
        match result {
            Err(ExtractError::NodeNotFound(kind)) => {
                assert_eq!(kind, node_types::OVERLAP_POINT);
            }
            _ => {
                return Err(
                    ExtractError::NodeNotFound(node_types::OVERLAP_POINT.to_string()).into(),
                )
            }
        }
        Ok(())
    }
}
