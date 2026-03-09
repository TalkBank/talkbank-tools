//! Debug tree functionality for this subsystem.
//!

use std::error::Error;
use std::io;
use tree_sitter::Parser;
use tree_sitter::TreeCursor;

/// Prints tree.
fn print_tree(
    cursor: &mut TreeCursor,
    source: &str,
    depth: usize,
) -> Result<(), std::str::Utf8Error> {
    let node = cursor.node();
    let indent = "  ".repeat(depth);
    let text = node
        .utf8_text(source.as_bytes())?
        .replace("\n", "\\n")
        .replace("\t", "\\t");
    let text_preview = if text.len() > 50 {
        format!("{}...", &text[..50])
    } else {
        text
    };
    println!(
        "{}{} [{}..{}] = {:?}",
        indent,
        node.kind(),
        node.start_byte(),
        node.end_byte(),
        text_preview
    );

    if cursor.goto_first_child() {
        print_tree(cursor, source, depth + 1)?;
        while cursor.goto_next_sibling() {
            print_tree(cursor, source, depth + 1)?;
        }
        cursor.goto_parent();
    }

    Ok(())
}

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let mut parser = Parser::new();
    let language = tree_sitter_talkbank::LANGUAGE.into();
    parser.set_language(&language)?;

    // Test the action.cha content
    let test = "@UTF8\n@Begin\n@Languages:\teng\n*CHI:\t0 [= did nothing] .\n@End\n";

    println!("Testing: {:?}\n", test);

    let tree = parser
        .parse(test, None)
        .ok_or_else(|| io::Error::other("Tree-sitter parse failed"))?;
    let mut cursor = tree.walk();
    print_tree(&mut cursor, test, 0)?;

    Ok(())
}
