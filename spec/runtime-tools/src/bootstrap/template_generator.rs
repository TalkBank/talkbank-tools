//! Generates scaffold templates from parsed CHAT fixtures.
//!
//! Given a fixture file and a target node kind, this module parses the file
//! with tree-sitter, replaces the target node's byte range with `{input}` in
//! the source and `{fragment}` in the CST, and produces a [`TemplateData`]
//! struct ready for YAML serialization.  The resulting template captures the
//! full surrounding CHAT context so that scaffold-generated specs contain
//! structurally valid input and expected CST pairs.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tree_sitter::Parser;

use super::cst_extractor::extract_cst_from_fixture;

/// Errors that can occur during template generation
#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("YAML error: {0}")]
    YamlError(#[from] serde_yaml_ng::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to extract CST from fixture: {0}")]
    CstExtract(#[from] super::cst_extractor::ExtractError),

    #[error("Failed to parse CHAT file with tree-sitter")]
    ParseFailed,

    #[error("Node '{0}' not found in CST")]
    NodeNotFound(String),
}

/// Template data for generating spec scaffolding
///
/// Contains both the input wrapper (CHAT file with {input} placeholder)
/// and CST wrapper (tree-sitter output with {fragment} placeholder).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateData {
    /// CHAT input with {input} placeholder where the fragment should be inserted
    pub input_wrapper: String,

    /// Tree-sitter CST output with {fragment} placeholder where the node should be inserted
    pub cst_wrapper: String,

    /// Metadata about the template generation
    pub metadata: TemplateMetadata,
}

/// Metadata about template generation
///
/// Tracks the source fixture, target node type, nesting level, and timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateMetadata {
    /// Path to the source fixture file used to generate this template
    pub source_fixture: String,

    /// The target tree-sitter node type this template is for
    pub target_node: String,

    /// Nesting level (indentation) for the fragment in the CST
    pub nesting_level: usize,

    /// Timestamp when the template was generated (RFC 3339 format)
    pub generated_at: String,
}

/// Generate input wrapper with {input} placeholder
///
/// Replaces target node's byte range with {input} placeholder
fn generate_input_wrapper(source: &str, start_byte: usize, end_byte: usize) -> String {
    let mut result = String::with_capacity(source.len() + 10);
    result.push_str(&source[..start_byte]);
    result.push_str("{input}");
    result.push_str(&source[end_byte..]);
    result
}

/// Calculate nesting depth of node in tree
fn calculate_nesting_level(mut node: tree_sitter::Node) -> usize {
    let mut depth = 0;
    while let Some(parent) = node.parent() {
        depth += 1;
        node = parent;
    }
    depth
}

/// Generate template data from fixture
///
/// # Arguments
/// * `fixture_path` - Path to fixture (for metadata)
/// * `source` - CHAT file content
/// * `target_kind` - Node kind to extract
///
/// # Returns
/// TemplateData ready for YAML serialization
pub fn generate_template(
    fixture_path: &Path,
    source: &str,
    target_kind: &str,
) -> Result<TemplateData, TemplateError> {
    // Extract CST with placeholder
    let cst_ir = extract_cst_from_fixture(source, target_kind)?;

    // Parse to find target node for byte positions and depth
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_talkbank::LANGUAGE.into())
        .map_err(|_| TemplateError::ParseFailed)?;

    let tree = parser
        .parse(source, None)
        .ok_or(TemplateError::ParseFailed)?;

    // Find target node
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

    let target_node = find_node(tree.root_node(), target_kind)
        .ok_or_else(|| TemplateError::NodeNotFound(target_kind.to_string()))?;

    // Generate input wrapper
    let input_wrapper =
        generate_input_wrapper(source, target_node.start_byte(), target_node.end_byte());

    // Serialize CST to string
    let cst_wrapper = cst_ir.to_string();

    // Calculate nesting level
    let nesting_level = calculate_nesting_level(target_node);

    // Generate metadata
    let metadata = TemplateMetadata {
        source_fixture: fixture_path.display().to_string(),
        target_node: target_kind.to_string(),
        nesting_level,
        generated_at: Utc::now().to_rfc3339(),
    };

    Ok(TemplateData {
        input_wrapper,
        cst_wrapper,
        metadata,
    })
}

/// Write template to YAML file
pub fn write_template_file(
    template: &TemplateData,
    output_path: &Path,
) -> Result<(), TemplateError> {
    let yaml = serde_yaml_ng::to_string(template)?;
    std::fs::write(output_path, yaml)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use talkbank_parser::node_types;

    /// Tests template data structure.
    #[test]
    fn test_template_data_structure() -> Result<()> {
        let template = TemplateData {
            input_wrapper: "*CHI:\t{input} .".to_string(),
            cst_wrapper: "(chat_file\n  {fragment}\n)".to_string(),
            metadata: TemplateMetadata {
                source_fixture: "fixtures/minimal.cha".to_string(),
                target_node: "word".to_string(),
                nesting_level: 2,
                generated_at: chrono::Utc::now().to_rfc3339(),
            },
        };

        assert_eq!(template.input_wrapper, "*CHI:\t{input} .");
        assert_eq!(template.cst_wrapper, "(chat_file\n  {fragment}\n)");
        assert_eq!(template.metadata.source_fixture, "fixtures/minimal.cha");
        assert_eq!(template.metadata.target_node, "word");
        assert_eq!(template.metadata.nesting_level, 2);
        assert!(!template.metadata.generated_at.is_empty());
        Ok(())
    }

    /// Tests serialize to yaml.
    #[test]
    fn test_serialize_to_yaml() -> Result<()> {
        let template = TemplateData {
            input_wrapper: "*CHI:\t{input} .".to_string(),
            cst_wrapper: "(chat_file\n  {fragment}\n)".to_string(),
            metadata: TemplateMetadata {
                source_fixture: "fixtures/test.cha".to_string(),
                target_node: "word".to_string(),
                nesting_level: 2,
                generated_at: "2026-01-12T10:00:00Z".to_string(),
            },
        };

        let yaml = serde_yaml_ng::to_string(&template)?;

        // Verify YAML contains expected fields
        assert!(yaml.contains("input_wrapper:"));
        assert!(yaml.contains("cst_wrapper:"));
        assert!(yaml.contains("metadata:"));
        assert!(yaml.contains("source_fixture:"));
        assert!(yaml.contains("target_node: word"));
        assert!(yaml.contains("nesting_level: 2"));
        assert!(yaml.contains("generated_at:"));

        // Verify it can be deserialized back
        let deserialized: TemplateData = serde_yaml_ng::from_str(&yaml)?;
        assert_eq!(deserialized, template);
        Ok(())
    }

    /// Tests generate input wrapper.
    #[test]
    fn test_generate_input_wrapper() -> Result<()> {
        let source = "@UTF8\n\
                      @Begin\n\
                      *CHI:\thello .\n\
                      @End\n";

        // Simulate target node at bytes 19-24 ("hello")
        let result = generate_input_wrapper(source, 19, 24);

        assert!(result.contains("{input}"));
        assert!(result.contains("@UTF8"));
        assert!(result.contains("*CHI:\t{input} ."));
        Ok(())
    }

    /// Tests generate input wrapper multiline.
    #[test]
    fn test_generate_input_wrapper_multiline() -> Result<()> {
        let source = "line1\nline2\ntarget\nline4\n";

        let result = generate_input_wrapper(source, 12, 18);

        assert_eq!(result, "line1\nline2\n{input}\nline4\n");
        Ok(())
    }

    /// Tests calculate nesting level.
    #[test]
    fn test_calculate_nesting_level() -> Result<()> {
        use tree_sitter::{Node, Parser};

        let source = "@UTF8\n\
                      @Begin\n\
                      *CHI:\thello .\n\
                      @End\n";

        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_talkbank::LANGUAGE.into())
            .map_err(|_| TemplateError::ParseFailed)?;
        let tree = parser
            .parse(source, None)
            .ok_or(TemplateError::ParseFailed)?;
        let root = tree.root_node();

        // Find standalone_word node (deeply nested)
        /// Finds word.
        fn find_word(node: Node) -> Option<Node> {
            if node.kind() == node_types::STANDALONE_WORD {
                return Some(node);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(found) = find_word(child) {
                    return Some(found);
                }
            }
            None
        }

        let word_node = find_word(root)
            .ok_or_else(|| TemplateError::NodeNotFound(node_types::STANDALONE_WORD.to_string()))?;
        let depth = calculate_nesting_level(word_node);

        // standalone_word is nested deep in CST
        assert!(depth > 5);
        Ok(())
    }

    /// Tests generate template.
    #[test]
    fn test_generate_template() -> Result<()> {
        use std::path::PathBuf;

        let source = "@UTF8\n\
                      @Begin\n\
                      *CHI:\thello .\n\
                      @End\n";

        let fixture_path = PathBuf::from("test.cha");
        let target_kind = node_types::STANDALONE_WORD;

        let template = generate_template(&fixture_path, source, target_kind)?;
        assert!(template.input_wrapper.contains("{input}"));
        assert!(template.cst_wrapper.contains("{fragment}"));
        assert_eq!(template.metadata.target_node, target_kind);
        assert!(template.metadata.nesting_level > 0);
        Ok(())
    }

    /// Tests write template file.
    #[test]
    fn test_write_template_file() -> Result<()> {
        use tempfile::tempdir;

        let template = TemplateData {
            input_wrapper: "*CHI:\t{input} .".to_string(),
            cst_wrapper: "(main_tier {fragment})".to_string(),
            metadata: TemplateMetadata {
                source_fixture: "test.cha".to_string(),
                target_node: "standalone_word".to_string(),
                nesting_level: 10,
                generated_at: "2026-01-11T22:00:00Z".to_string(),
            },
        };

        let dir = tempdir()?;
        let output_path = dir.path().join("test_template.yaml");

        write_template_file(&template, &output_path)?;

        // Verify file was written
        assert!(output_path.exists());

        // Verify content is valid YAML
        let content = std::fs::read_to_string(&output_path)?;
        assert!(content.contains("input_wrapper"));
        assert!(content.contains("{input}"));
        Ok(())
    }
}
