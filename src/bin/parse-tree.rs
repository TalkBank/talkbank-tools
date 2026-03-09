//! Quick tree-sitter parse-tree inspector for ad-hoc CHAT snippets.
//!

use std::env;

/// Parse a snippet in minimal CHAT context and print CST structure plus error stats.
fn main() {
    let args: Vec<String> = env::args().collect();
    let raw_input = if args.len() > 1 {
        args[1..].join(" ")
    } else {
        eprintln!("Usage: parse-tree '<CHAT snippet>'");
        eprintln!("Example: parse-tree '%xphon:\\t$OMI $OMI'");
        std::process::exit(1);
    };

    // Ensure input ends with newline
    let input = if raw_input.ends_with('\n') {
        raw_input.clone()
    } else {
        format!("{}\n", raw_input)
    };

    // Wrap in minimal CHAT file for parsing
    let wrapped = format!(
        "@UTF8\n\
         @Begin\n\
         @Participants:\tCHI Target_Child\n\
         *CHI:\tdummy .\n\
         {}\
         @End\n",
        input
    );

    let mut parser = tree_sitter::Parser::new();
    if let Err(err) = parser.set_language(&tree_sitter_talkbank::LANGUAGE.into()) {
        eprintln!("Failed to set tree-sitter language: {err}");
        std::process::exit(1);
    }

    let Some(tree) = parser.parse(&wrapped, None) else {
        eprintln!("Failed to parse input");
        std::process::exit(1);
    };
    let root = tree.root_node();

    println!("=== Parse Tree: \"{}\" ===\n", raw_input);
    print_tree(root, &wrapped, 0);

    // Check for errors
    let (has_errors, error_count) = check_errors(root);

    println!();
    if has_errors {
        println!("Status: ❌ {} errors found", error_count);
    } else {
        println!("Status: ✅ No errors");
    }

    println!("\nNode count: {}", count_nodes(root));
    println!("Depth: {}", tree_depth(root, 0));
}

/// Recursively print node kind/range and short leaf text for a CST subtree.
fn print_tree(node: tree_sitter::Node, source: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    let kind = node.kind();
    let start = node.start_byte();
    let end = node.end_byte();

    // Get text if it's a leaf or small node
    let text = if node.child_count() == 0 || end - start < 50 {
        node.utf8_text(source.as_bytes())
            // DEFAULT: Invalid UTF-8 is rendered as a fixed placeholder in the tree dump.
            .unwrap_or("<invalid-utf8>")
    } else {
        ""
    };

    // Highlight errors
    let prefix = if node.is_error() {
        "⚠️ ERROR "
    } else if node.is_missing() {
        "⚠️ MISSING "
    } else {
        ""
    };

    if text.is_empty() {
        println!("{}{}{} [{}..{}]", indent, prefix, kind, start, end);
    } else {
        let display_text = text.replace('\n', "\\n").replace('\t', "\\t");
        println!(
            "{}{}{} [{}..{}] \"{}\"",
            indent, prefix, kind, start, end, display_text
        );
    }

    // Print children
    if node.child_count() > 0 {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            print_tree(child, source, depth + 1);
        }
    }
}

/// Recursively detect `ERROR`/`MISSING` nodes and return `(has_errors, count)`.
fn check_errors(node: tree_sitter::Node) -> (bool, usize) {
    let mut has_errors = node.is_error() || node.is_missing();
    let mut count = if has_errors { 1 } else { 0 };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let (child_has_errors, child_count) = check_errors(child);
        if child_has_errors {
            has_errors = true;
            count += child_count;
        }
    }

    (has_errors, count)
}

/// Count total nodes in a subtree.
fn count_nodes(node: tree_sitter::Node) -> usize {
    let mut count = 1;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        count += count_nodes(child);
    }
    count
}

/// Compute maximum depth reachable from the current node.
fn tree_depth(node: tree_sitter::Node, current_depth: usize) -> usize {
    if node.child_count() == 0 {
        return current_depth;
    }

    let mut max_depth = current_depth;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let child_depth = tree_depth(child, current_depth + 1);
        if child_depth > max_depth {
            max_depth = child_depth;
        }
    }
    max_depth
}
