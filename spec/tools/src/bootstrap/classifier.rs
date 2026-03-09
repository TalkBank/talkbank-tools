//! Loads the user-edited `node_config.yaml` and provides query methods for
//! filtering, counting, and looking up template assignments.
//!
//! After the analyzer generates `all_nodes_annotated.yaml`, the user copies it
//! to `node_config.yaml` and edits the `test:` flags.  This module deserializes
//! that file into a [`NodeConfig`] and exposes the subset of nodes the user
//! actually wants to scaffold.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur while loading `node_config.yaml`.
#[derive(Debug, Error)]
pub enum NodeConfigError {
    #[error("Failed to read config file: {path}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("Failed to parse YAML: {path}")]
    Parse {
        path: String,
        source: serde_yaml_ng::Error,
    },
}

/// Node configuration loaded from YAML
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeConfig {
    pub nodes: HashMap<String, NodeEntry>,
}

/// A single node's configuration as written in `node_config.yaml`.
///
/// The analyzer generates sensible defaults for each field, but the user is
/// expected to review and override them before running the scaffolder.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeEntry {
    /// Whether to generate a spec file for this node.  `true` means the node
    /// will be included in the scaffolding run; `false` means it is skipped.
    pub test: bool,
    /// Name of the scaffold template to apply (e.g. `"word_level"`,
    /// `"document"`).  `None` is typical for skipped nodes and means no
    /// template is needed.  When `test` is `true` but `template` is `None`,
    /// the scaffolder falls back to using the node name itself as the template
    /// name.
    pub template: Option<String>,
    /// Free-form priority label (`"critical"`, `"high"`, `"medium"`, `"low"`)
    /// carried over from the analyzer's classification.  Used only for display
    /// and YAML comments; the scaffolder does not interpret it.
    pub priority: String,
    /// Human-readable justification for the test/skip decision, displayed as a
    /// YAML comment in the config file.
    pub reason: String,
}

impl NodeConfig {
    /// Load configuration from YAML file
    ///
    /// # Arguments
    /// * `path` - Path to node_config.yaml file
    ///
    /// # Returns
    /// * `Ok(NodeConfig)` - Loaded configuration
    /// * `Err(String)` - Error message if loading fails
    pub fn load(path: &Path) -> Result<Self, NodeConfigError> {
        let content = fs::read_to_string(path).map_err(|source| NodeConfigError::Read {
            path: path.display().to_string(),
            source,
        })?;

        let config: NodeConfig =
            serde_yaml_ng::from_str(&content).map_err(|source| NodeConfigError::Parse {
                path: path.display().to_string(),
                source,
            })?;

        Ok(config)
    }

    /// Get list of nodes marked for testing
    ///
    /// # Returns
    /// * `Vec<String>` - List of node names where test=true
    pub fn nodes_to_test(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter(|(_, entry)| entry.test)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get template name for a node
    ///
    /// # Arguments
    /// * `node` - Node name
    ///
    /// # Returns
    /// * `Some(String)` - Template name if specified, or node name as default
    /// * `None` - If node not in config
    pub fn template_for_node(&self, node: &str) -> Option<String> {
        self.nodes
            .get(node)
            .map(|entry| match entry.template.clone() {
                Some(template) => template,
                None => node.to_string(),
            })
    }

    /// Get total count of nodes in config
    pub fn total_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Get count of nodes marked for testing
    pub fn test_count(&self) -> usize {
        self.nodes.iter().filter(|(_, entry)| entry.test).count()
    }

    /// Get count of nodes marked to skip
    pub fn skip_count(&self) -> usize {
        self.nodes.iter().filter(|(_, entry)| !entry.test).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    /// Tests load config.
    #[test]
    fn test_load_config() -> Result<()> {
        // Create temporary config file
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_node_config.yaml");

        let yaml_content = r#"
nodes:
  document:
    test: true
    template: "document"
    priority: critical
    reason: "Root node"

  standalone_word:
    test: true
    template: "word_level"
    priority: high
    reason: "Word construct"

  word_segment:
    test: false
    template: null
    priority: low
    reason: "Too granular"
"#;

        fs::write(&config_path, yaml_content)?;

        // Load config
        let config = NodeConfig::load(&config_path)?;

        // Verify loaded correctly
        assert_eq!(config.total_nodes(), 3);
        assert_eq!(config.test_count(), 2);
        assert_eq!(config.skip_count(), 1);

        // Verify specific nodes
        let doc_entry = config
            .nodes
            .get("document")
            .ok_or_else(|| anyhow::anyhow!("Missing document node"))?;
        assert!(doc_entry.test);
        assert_eq!(doc_entry.template, Some("document".to_string()));
        assert_eq!(doc_entry.priority, "critical");

        let segment_entry = config
            .nodes
            .get("word_segment")
            .ok_or_else(|| anyhow::anyhow!("Missing word_segment node"))?;
        assert!(!segment_entry.test);
        assert_eq!(segment_entry.template, None);

        // Verify nodes_to_test
        let test_nodes = config.nodes_to_test();
        assert_eq!(test_nodes.len(), 2);
        assert!(test_nodes.contains(&"document".to_string()));
        assert!(test_nodes.contains(&"standalone_word".to_string()));

        // Verify template_for_node
        assert_eq!(
            config.template_for_node("document"),
            Some("document".to_string())
        );
        assert_eq!(
            config.template_for_node("standalone_word"),
            Some("word_level".to_string())
        );

        // Clean up
        let _ = fs::remove_file(config_path);
        Ok(())
    }

    /// Tests load invalid yaml.
    #[test]
    fn test_load_invalid_yaml() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_invalid_config.yaml");

        let invalid_yaml = "{ invalid yaml content [[[";
        fs::write(&config_path, invalid_yaml)?;

        let result = NodeConfig::load(&config_path);
        assert!(result.is_err());
        assert!(matches!(result, Err(NodeConfigError::Parse { .. })));

        let _ = fs::remove_file(config_path);
        Ok(())
    }

    /// Tests load missing file.
    #[test]
    fn test_load_missing_file() -> Result<()> {
        let nonexistent_path = Path::new("/nonexistent/path/to/config.yaml");
        let result = NodeConfig::load(nonexistent_path);
        assert!(result.is_err());
        assert!(matches!(result, Err(NodeConfigError::Read { .. })));
        Ok(())
    }
}
