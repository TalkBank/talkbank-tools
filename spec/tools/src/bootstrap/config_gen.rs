//! Generates `all_nodes_annotated.yaml` from a list of [`NodeClassification`]s.
//!
//! This is the bridge between the analyzer (which classifies every grammar node)
//! and the user-editable `node_config.yaml`.  The generated file contains all
//! 500+ named nodes grouped by semantic category, each with pre-filled `test:`,
//! `template:`, `priority:`, and `reason:` fields plus inline comments
//! explaining the classification rationale.  The user copies this file, reviews
//! the suggestions, and edits as needed before running the scaffolder.

use super::analyzer::{NodeClassification, TestSuggestion};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur while writing the bootstrap config file.
#[derive(Debug, Error)]
pub enum ConfigGenError {
    #[error("Failed to write config file: {path}")]
    Write {
        path: String,
        source: std::io::Error,
    },
}

/// Generate bootstrap configuration YAML file
///
/// Creates all_nodes_annotated.yaml with:
/// - Header comments explaining the file and workflow
/// - All nodes grouped by category
/// - Each node with test:, template:, priority:, reason: fields
/// - Inline comments for guidance
///
/// # Arguments
/// * `classifications` - List of all classified nodes
/// * `output_path` - Path to write all_nodes_annotated.yaml
pub fn generate_bootstrap_config(
    classifications: &[NodeClassification],
    output_path: &Path,
) -> Result<(), ConfigGenError> {
    // Group nodes by category for organized output
    let grouped = group_by_category(classifications);

    // Generate YAML content with comments
    let mut yaml = String::new();

    // Header
    yaml.push_str(&generate_header());

    // Nodes section
    yaml.push_str("nodes:\n");

    // Add each category group
    for (category, nodes) in &grouped {
        yaml.push_str(&format!("\n  # {}\n", format_category_header(category)));
        yaml.push_str(&format!("  # {}\n", "=".repeat(70)));
        yaml.push('\n');

        for node in nodes {
            yaml.push_str(&format_node_entry(node));
        }
    }

    // Write to file
    fs::write(output_path, yaml).map_err(|source| ConfigGenError::Write {
        path: output_path.display().to_string(),
        source,
    })?;

    Ok(())
}

/// Generate file header with instructions
fn generate_header() -> String {
    r#"# ============================================================================
# CHAT Node Test Configuration (Bootstrap)
# ============================================================================
# Generated from grammar.js - ALL 500+ nodes listed with test suggestions
#
# INSTRUCTIONS:
# 1. Copy this file to node_config.yaml
# 2. Review each node's test: true/false suggestion
# 3. Edit to mark which nodes you actually want to test
# 4. Run: cargo run --bin scaffold_nodes
#
# CLASSIFICATION LEGEND:
# - priority: critical  → Core CHAT structure (MUST test)
# - priority: high      → Important constructs (SHOULD test)
# - priority: medium    → Optional constructs (COULD test)
# - priority: low       → Rarely used (probably skip)
# - test: false         → Suggested skip (too granular, trivial, or validation)
# ============================================================================

"#
    .to_string()
}

/// Group nodes by category for organized output
fn group_by_category(
    classifications: &[NodeClassification],
) -> Vec<(Category, Vec<&NodeClassification>)> {
    let mut categories: HashMap<Category, Vec<&NodeClassification>> = HashMap::new();

    for node in classifications {
        let category = categorize_node(node);
        categories.entry(category).or_default().push(node);
    }

    // Sort categories in priority order
    let mut grouped: Vec<_> = categories.into_iter().collect();
    grouped.sort_by_key(|(cat, _)| category_priority(cat));

    grouped
}

/// Semantic categories used to group nodes in the generated YAML config.
///
/// Nodes are bucketed by name patterns and classification results so the user
/// sees related nodes together.  Category ordering (via [`category_priority`])
/// puts the most important groups at the top of the file.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Category {
    /// Skeleton nodes that define every CHAT file: `document`, `line`,
    /// `utterance`, `main_tier`, `dependent_tier`.
    CoreStructure,
    /// `@`-prefixed header lines (e.g. `@Languages`, `@Participants`).
    Headers,
    /// Nodes containing actual spoken or written words (`standalone_word`,
    /// `word_with_optional_annotations`, `mor_word`, etc.).
    Words,
    /// Typed dependent tier nodes (`%mor`, `%gra`, `%pho`, etc.).
    DependentTiers,
    /// Annotation constructs attached to words (e.g. `alt_annotation`,
    /// `error_marker_annotation`).
    Annotations,
    /// Event and action markers (`event`, `other_spoken_event`).
    EventsActions,
    /// Punctuation tokens, whitespace literals, and low-level markers that
    /// tree-sitter handles correctly without dedicated tests.
    TrivialTerminals,
    /// Internal sub-word or sub-tier components too fine-grained to warrant
    /// individual spec tests.
    SubComponents,
    /// Anything that does not match the above patterns.
    Other,
}

/// Determine category for a node
fn categorize_node(node: &NodeClassification) -> Category {
    let name = &node.name;

    if matches!(
        name.as_str(),
        "document" | "line" | "utterance" | "main_tier" | "dependent_tier"
    ) {
        return Category::CoreStructure;
    }

    if name.ends_with("_header") {
        return Category::Headers;
    }

    if name.ends_with("_dependent_tier") {
        return Category::DependentTiers;
    }

    if name.contains("word") && !name.contains("segment") {
        return Category::Words;
    }

    if name.ends_with("_annotation") {
        return Category::Annotations;
    }

    if name.contains("event") || name.contains("action") {
        return Category::EventsActions;
    }

    if matches!(node.suggestion, TestSuggestion::Skip)
        && (name.ends_with("_segment")
            || name.ends_with("_token")
            || name.ends_with("_prefix")
            || name.ends_with("_suffix")
            || name.ends_with("_marker")
            || matches!(
                name.as_str(),
                "period" | "comma" | "space" | "tab" | "newline" | "star" | "colon"
            ))
    {
        return Category::TrivialTerminals;
    }

    if matches!(node.suggestion, TestSuggestion::Skip) {
        return Category::SubComponents;
    }

    Category::Other
}

/// Category display priority (lower = earlier in file)
fn category_priority(category: &Category) -> u8 {
    match category {
        Category::CoreStructure => 0,
        Category::Headers => 1,
        Category::Words => 3,
        Category::DependentTiers => 4,
        Category::Annotations => 5,
        Category::EventsActions => 6,
        Category::Other => 7,
        Category::SubComponents => 8,
        Category::TrivialTerminals => 9,
    }
}

/// Format category as header string
fn format_category_header(category: &Category) -> String {
    match category {
        Category::CoreStructure => "Core Document Structure (MUST TEST)".to_string(),
        Category::Headers => "Headers (SHOULD TEST)".to_string(),
        Category::Words => "Word-Level Nodes (SHOULD TEST)".to_string(),
        Category::DependentTiers => "Dependent Tiers (SHOULD TEST)".to_string(),
        Category::Annotations => "Annotations (SHOULD TEST)".to_string(),
        Category::EventsActions => "Events and Actions (SHOULD TEST)".to_string(),
        Category::Other => "Other Constructs (COULD TEST)".to_string(),
        Category::SubComponents => "Sub-Components (SKIP - Too Granular)".to_string(),
        Category::TrivialTerminals => "Trivial Terminals (SKIP)".to_string(),
    }
}

/// Format a single node entry with YAML fields and comments
fn format_node_entry(node: &NodeClassification) -> String {
    let test_flag = node.test_flag();
    let template_str = match node.template.as_ref() {
        Some(template) => format!("\"{}\"", template),
        None => "null".to_string(),
    };

    format!(
        "  {}:
    test: {}  # {}
    template: {}
    priority: {}
    reason: \"{}\"\n\n",
        node.name,
        test_flag,
        priority_comment(&node.priority),
        template_str,
        node.priority,
        node.reason
    )
}

/// Generate inline comment for priority field
fn priority_comment(priority: &str) -> &'static str {
    match priority {
        "critical" => "Core structure - MUST test",
        "high" => "Important construct - SHOULD test",
        "medium" => "Optional - COULD test",
        "low" => "Skip - too granular/trivial",
        _ => "Priority not set",
    }
}

#[cfg(test)]
mod tests {
    use super::super::analyzer::TestSuggestion;
    use super::*;
    use anyhow::Result;

    /// Tests generate bootstrap config.
    #[test]
    fn test_generate_bootstrap_config() -> Result<()> {
        let classifications = vec![
            NodeClassification {
                name: "document".to_string(),
                suggestion: TestSuggestion::MustTest,
                reason: "Core structure".to_string(),
                template: Some("document".to_string()),
                priority: "critical".to_string(),
            },
            NodeClassification {
                name: "standalone_word".to_string(),
                suggestion: TestSuggestion::ShouldTest,
                reason: "Word type".to_string(),
                template: Some("word_level".to_string()),
                priority: "high".to_string(),
            },
            NodeClassification {
                name: "period".to_string(),
                suggestion: TestSuggestion::Skip,
                reason: "Trivial terminal".to_string(),
                template: None,
                priority: "low".to_string(),
            },
        ];

        let output_path = std::env::temp_dir().join("test_bootstrap_config.yaml");
        generate_bootstrap_config(&classifications, &output_path)?;

        // Verify file was created
        assert!(output_path.exists());

        // Read and verify content
        let content = fs::read_to_string(&output_path)?;

        // Check header is present
        assert!(content.contains("CHAT Node Test Configuration"));
        assert!(content.contains("INSTRUCTIONS:"));
        assert!(content.contains("CLASSIFICATION LEGEND:"));

        // Check nodes section
        assert!(content.contains("nodes:"));

        // Check document node
        assert!(content.contains("document:"));
        assert!(content.contains("test: true"));
        assert!(content.contains("template: \"document\""));
        assert!(content.contains("priority: critical"));

        // Check standalone_word node
        assert!(content.contains("standalone_word:"));
        assert!(content.contains("template: \"word_level\""));

        // Check period node
        assert!(content.contains("period:"));
        assert!(content.contains("test: false"));
        assert!(content.contains("template: null"));

        // Clean up
        let _ = fs::remove_file(output_path);
        Ok(())
    }

    /// Tests categorize nodes.
    #[test]
    fn test_categorize_nodes() -> Result<()> {
        let node = NodeClassification {
            name: "document".to_string(),
            suggestion: TestSuggestion::MustTest,
            reason: "".to_string(),
            template: None,
            priority: "critical".to_string(),
        };
        assert_eq!(categorize_node(&node), Category::CoreStructure);

        let node = NodeClassification {
            name: "languages_header".to_string(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "".to_string(),
            template: None,
            priority: "high".to_string(),
        };
        assert_eq!(categorize_node(&node), Category::Headers);

        let node = NodeClassification {
            name: "standalone_word".to_string(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "".to_string(),
            template: None,
            priority: "high".to_string(),
        };
        assert_eq!(categorize_node(&node), Category::Words);

        let node = NodeClassification {
            name: "mor_dependent_tier".to_string(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "".to_string(),
            template: None,
            priority: "high".to_string(),
        };
        assert_eq!(categorize_node(&node), Category::DependentTiers);
        Ok(())
    }
}
