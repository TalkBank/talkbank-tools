//! Test module for test pause parsing in `talkbank-tools`.
//!
//! These tests document expected behavior and regressions.

use std::error::Error;
use std::io;
use talkbank_parser::node_types::PAUSE_TOKEN;
use tree_sitter::Parser;

/// Parses tree.
fn parse_tree(parser: &mut Parser, content: &str) -> Result<tree_sitter::Tree, io::Error> {
    parser
        .parse(content, None)
        .ok_or_else(|| io::Error::other("Tree-sitter parse failed"))
}

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let mut parser = Parser::new();
    let language = tree_sitter_talkbank::LANGUAGE.into();
    parser.set_language(&language)?;

    let test = "@UTF8\n@Begin\n*CHI:\t(0.4) .\n@End\n";
    println!("Testing: {:?}", test);

    let tree = parse_tree(&mut parser, test)?;

    // Navigate to pause_token
    /// Finds node.
    fn find_node<'a>(node: tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
        if node.kind() == kind {
            return Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = find_node(child, kind) {
                return Some(found);
            }
        }
        None
    }

    if let Some(pause_token) = find_node(tree.root_node(), PAUSE_TOKEN) {
        println!("\npause_token found (atomic leaf):");
        println!("  child_count: {}", pause_token.child_count());
        println!("  text: {:?}", pause_token.utf8_text(test.as_bytes()));
    }

    Ok(())
}
