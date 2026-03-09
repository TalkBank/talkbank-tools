//! Debug the header_media parsing issue

use std::error::Error;
use std::io;
use tree_sitter::Parser;

/// Prints tree.
fn print_tree(
    node: tree_sitter::Node,
    source: &str,
    depth: usize,
) -> Result<(), std::str::Utf8Error> {
    let indent = "  ".repeat(depth);
    let text = node.utf8_text(source.as_bytes())?;
    let text_preview = if text.len() > 50 {
        format!("{}...", &text[..50])
    } else {
        text.to_string()
    };
    let text_preview = text_preview.replace('\n', "\\n").replace('\t', "\\t");
    println!(
        "{}{} [{}..{}] = \"{}\"",
        indent,
        node.kind(),
        node.start_byte(),
        node.end_byte(),
        text_preview
    );

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i as u32) {
            print_tree(child, source, depth + 1)?;
        }
    }

    Ok(())
}

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let wrapped = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Target_Child\n@ID:\teng|corpus|CHI|||||Target_Child|||\n@Media:\thttps://media.talkbank.org/childes/Eng-NA/MacWhinney/000203a.mp3, video\n@End\n";

    println!("Input:");
    println!("{}", wrapped);
    println!("\n---\n");

    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_talkbank::LANGUAGE.into())?;

    let tree = parser
        .parse(wrapped, None)
        .ok_or_else(|| io::Error::other("Tree-sitter parse failed"))?;
    let root = tree.root_node();

    print_tree(root, wrapped, 0)?;

    Ok(())
}
