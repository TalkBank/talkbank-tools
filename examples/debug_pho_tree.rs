//! Debug pho tree functionality for this subsystem.
//!

use std::error::Error;
use std::io;
use tree_sitter::Parser;

/// Prints tree.
fn print_tree(node: tree_sitter::Node, source: &str, indent: usize) {
    let text = &source[node.start_byte()..node.end_byte()];
    println!(
        "{:indent$}{} ({}) - {:?}",
        "",
        node.kind(),
        node.child_count(),
        text,
        indent = indent * 2
    );

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_tree(child, source, indent + 1);
    }
}

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let mut parser = Parser::new();
    let language = tree_sitter_talkbank::LANGUAGE.into();
    parser.set_language(&language)?;

    let test = "@UTF8\n@Begin\n*CHI:\t‹est lait› .\n@End\n";

    println!("Input: {:?}\n", test);

    let tree = parser
        .parse(test, None)
        .ok_or_else(|| io::Error::other("Tree-sitter parse failed"))?;
    let root = tree.root_node();

    print_tree(root, test, 0);

    Ok(())
}
