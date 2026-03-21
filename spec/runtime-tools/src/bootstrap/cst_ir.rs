//! Custom intermediate representation for tree-sitter CST
//!
//! Represents CST structure with placeholders for template generation.

use serde::Serialize;
use std::fmt::{self, Display, Formatter};

/// Intermediate representation of a tree-sitter CST with one node replaced by
/// a named placeholder.
///
/// The IR mirrors the tree-sitter node structure (kind + span + children) but
/// is detached from the tree-sitter lifetime so it can be serialized, compared,
/// and rendered independently.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum CstIr {
    /// A concrete CST node with its tree-sitter kind, source span, and
    /// recursively represented children.
    Node {
        /// Tree-sitter node kind (e.g. `"document"`, `"main_tier"`).
        kind: String,
        /// Start position as `(row, column)`, both zero-indexed.
        start: (usize, usize),
        /// End position as `(row, column)`, both zero-indexed.
        end: (usize, usize),
        /// Child nodes in source order.  Leaf nodes have an empty vec.
        children: Vec<CstIr>,
    },
    /// A named placeholder (typically `"{fragment}"`) that marks where the
    /// target node was in the original CST.  During template application the
    /// placeholder is replaced with the actual node-specific CST.
    Placeholder(String),
}

impl Display for CstIr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.write_indented(f, 0)
    }
}

impl CstIr {
    /// Recursively writes the S-expression representation with `level` levels
    /// of two-space indentation for child nodes.
    fn write_indented(&self, f: &mut Formatter, level: usize) -> fmt::Result {
        match self {
            CstIr::Placeholder(text) => write!(f, "{}", text),
            CstIr::Node {
                kind,
                start,
                end,
                children,
            } => {
                write!(
                    f,
                    "({} [{}, {}] - [{}, {}]",
                    kind, start.0, start.1, end.0, end.1
                )?;

                for child in children {
                    write!(f, "\n{}", "  ".repeat(level + 1))?;
                    child.write_indented(f, level + 1)?;
                }

                write!(f, ")")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests placeholder display.
    #[test]
    fn test_placeholder_display() {
        let placeholder = CstIr::Placeholder("{fragment}".to_string());
        assert_eq!(placeholder.to_string(), "{fragment}");
    }

    /// Tests node without children.
    #[test]
    fn test_node_without_children() {
        let node = CstIr::Node {
            kind: "word".to_string(),
            start: (6, 6),
            end: (6, 11),
            children: vec![],
        };
        assert_eq!(node.to_string(), "(word [6, 6] - [6, 11])");
    }

    /// Tests node with children.
    #[test]
    fn test_node_with_children() {
        let node = CstIr::Node {
            kind: "main_tier".to_string(),
            start: (6, 0),
            end: (7, 0),
            children: vec![
                CstIr::Node {
                    kind: "star".to_string(),
                    start: (6, 0),
                    end: (6, 1),
                    children: vec![],
                },
                CstIr::Placeholder("{fragment}".to_string()),
            ],
        };

        let expected = "(main_tier [6, 0] - [7, 0]\n  (star [6, 0] - [6, 1])\n  {fragment})";
        assert_eq!(node.to_string(), expected);
    }

    /// Tests nested children indentation.
    #[test]
    fn test_nested_children_indentation() {
        let node = CstIr::Node {
            kind: "document".to_string(),
            start: (0, 0),
            end: (8, 0),
            children: vec![CstIr::Node {
                kind: "line".to_string(),
                start: (6, 0),
                end: (7, 0),
                children: vec![CstIr::Node {
                    kind: "utterance".to_string(),
                    start: (6, 0),
                    end: (7, 0),
                    children: vec![CstIr::Placeholder("{fragment}".to_string())],
                }],
            }],
        };

        let output = node.to_string();
        assert!(output.contains("(document [0, 0] - [8, 0]"));
        assert!(output.contains("  (line [6, 0] - [7, 0]"));
        assert!(output.contains("    (utterance [6, 0] - [7, 0]"));
        assert!(output.contains("      {fragment}"));
    }
}
