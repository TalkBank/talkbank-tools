//! Orchestrator that drives the end-to-end spec scaffolding pipeline.
//!
//! Given a user-edited `node_config.yaml` and a directory of YAML templates,
//! the [`Scaffolder`] iterates over every node marked `test: true`, resolves
//! its template (with inheritance), fills in example input from the example
//! library, and writes a markdown spec file to the output directory.
//!
//! Supports a dry-run mode that reports what would be generated without writing
//! any files.

use super::classifier::{NodeConfig, NodeConfigError};
use super::examples;
use super::template::{TemplateError, TemplateLoader};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

/// Enum variants for ScaffoldError.
#[derive(Debug, Error)]
pub enum ScaffoldError {
    #[error("Failed to load node config")]
    Config(#[from] NodeConfigError),
    #[error("Failed to load template")]
    Template(#[from] TemplateError),
    #[error("Failed to create output directory: {path}")]
    CreateOutputDir {
        path: String,
        source: std::io::Error,
    },
    #[error("Failed to write spec file: {path}")]
    WriteSpec {
        path: String,
        source: std::io::Error,
    },
    #[error("Node not found in config: {node}")]
    MissingNode { node: String },
}

/// Scaffold configuration
pub struct Scaffolder {
    /// Path to node_config.yaml (user-edited)
    pub config_path: PathBuf,
    /// Directory containing template YAML files
    pub template_dir: PathBuf,
    /// Output directory for generated spec files
    pub output_dir: PathBuf,
}

/// Summary returned by [`Scaffolder::scaffold`] after a scaffolding run.
///
/// Counts reflect the full node config, not just the nodes marked for testing.
#[derive(Debug, Clone)]
pub struct ScaffoldReport {
    /// Total number of nodes in the loaded `node_config.yaml`, regardless of
    /// their `test` flag.
    pub total_nodes: usize,
    /// Number of spec files successfully generated (or that would be generated
    /// in dry-run mode).
    pub generated: usize,
    /// Number of test-eligible nodes that were skipped because their template
    /// could not be resolved or another non-fatal error occurred.
    pub skipped: usize,
    /// Human-readable error messages for nodes that failed generation.  Empty
    /// on a fully successful run.
    pub errors: Vec<String>,
}

impl Scaffolder {
    /// Create new scaffolder
    pub fn new(config_path: PathBuf, template_dir: PathBuf, output_dir: PathBuf) -> Self {
        Self {
            config_path,
            template_dir,
            output_dir,
        }
    }

    /// Run scaffolding pipeline
    ///
    /// # Arguments
    /// * `dry_run` - If true, don't write files, just report what would be done
    ///
    /// # Returns
    /// * `Ok(ScaffoldReport)` - Summary of what was generated
    /// * `Err(String)` - Fatal error message
    pub fn scaffold(&self, dry_run: bool) -> Result<ScaffoldReport, ScaffoldError> {
        println!("Loading configuration: {}", self.config_path.display());

        // Step 1: Load node configuration
        let config = NodeConfig::load(&self.config_path)?;
        println!("✓ Loaded {} nodes from config", config.total_nodes());
        println!("  {} nodes marked for testing", config.test_count());

        // Step 2: Get nodes to test
        let nodes_to_test = config.nodes_to_test();
        println!("\nGenerating specs for {} nodes...", nodes_to_test.len());

        // Step 3: Create template loader
        let mut template_loader = TemplateLoader::new(self.template_dir.clone());

        // Step 4: Create output directory
        if !dry_run {
            fs::create_dir_all(&self.output_dir).map_err(|source| {
                ScaffoldError::CreateOutputDir {
                    path: self.output_dir.display().to_string(),
                    source,
                }
            })?;
        }

        // Step 5: Generate spec for each node
        let mut generated = 0;
        let mut skipped = 0;
        let mut errors = Vec::new();

        for node_name in &nodes_to_test {
            match self.generate_spec(node_name, &config, &mut template_loader, dry_run) {
                Ok(true) => generated += 1,
                Ok(false) => skipped += 1,
                Err(e) => {
                    errors.push(format!("Error generating spec for {}: {}", node_name, e));
                    skipped += 1;
                }
            }
        }

        Ok(ScaffoldReport {
            total_nodes: config.total_nodes(),
            generated,
            skipped,
            errors,
        })
    }

    /// Generate spec file for a single node
    ///
    /// # Returns
    /// * `Ok(true)` - Spec generated successfully
    /// * `Ok(false)` - Spec skipped (no template, etc.)
    /// * `Err(String)` - Error generating spec
    fn generate_spec(
        &self,
        node_name: &str,
        config: &NodeConfig,
        template_loader: &mut TemplateLoader,
        dry_run: bool,
    ) -> Result<bool, ScaffoldError> {
        // Get template name for this node
        let template_name = match config.template_for_node(node_name) {
            Some(name) => name,
            None => {
                return Err(ScaffoldError::MissingNode {
                    node: node_name.to_string(),
                })
            }
        };

        // Load and resolve template
        let template = template_loader.load(&template_name)?;

        // Get example input from library (node-specific realistic examples)
        let input = examples::get_example_input(node_name).to_string();

        // Generate placeholder CST fragment
        // TODO: In future, use curate-cst to extract real CST
        let fragment_cst = format!("({})", node_name);

        // Apply template
        let (wrapped_input, wrapped_cst) = template.apply(&input, &fragment_cst);

        // Generate markdown spec
        let spec_content = format_spec_markdown(
            node_name,
            &template.metadata.description,
            node_name, // fence type
            &wrapped_input,
            &wrapped_cst,
            &template_name,
        );

        // Write to file
        let output_path = self.output_dir.join(format!("{}_example.md", node_name));

        if dry_run {
            println!("  [DRY RUN] Would generate: {}", output_path.display());
        } else {
            fs::write(&output_path, spec_content).map_err(|source| ScaffoldError::WriteSpec {
                path: output_path.display().to_string(),
                source,
            })?;
            println!("  ✓ Generated: {}", output_path.display());
        }

        Ok(true)
    }
}

/// Format spec as markdown
fn format_spec_markdown(
    node_name: &str,
    description: &str,
    fence_type: &str,
    input: &str,
    cst: &str,
    template: &str,
) -> String {
    let desc = if description.is_empty() {
        format!("Example for {}", node_name)
    } else {
        description.to_string()
    };

    format!(
        r#"# {}_example

{}

## Input

``` {}
{}
```

## Expected CST

``` cst
{}
```

## Metadata

- **Level**: node
- **Category**: scaffolded
- **Template**: {}
"#,
        node_name, desc, fence_type, input, cst, template
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    /// Tests format spec markdown.
    #[test]
    fn test_format_spec_markdown() -> Result<()> {
        let md = format_spec_markdown(
            "standalone_word",
            "Word-level construct",
            "standalone_word",
            "*CHI:\thello .",
            "(standalone_word\n  (word_body))",
            "word_level",
        );

        assert!(md.contains("# standalone_word_example"));
        assert!(md.contains("Word-level construct"));
        assert!(md.contains("``` standalone_word"));
        assert!(md.contains("``` cst"));
        assert!(md.contains("**Template**: word_level"));
        Ok(())
    }

    /// Tests scaffolder dry run.
    #[test]
    fn test_scaffolder_dry_run() -> Result<()> {
        // Create temporary config and templates
        let temp_dir = std::env::temp_dir().join("test_scaffold");
        let config_dir = temp_dir.join("config");
        let template_dir = temp_dir.join("templates");
        let output_dir = temp_dir.join("output");

        fs::create_dir_all(&config_dir)?;
        fs::create_dir_all(&template_dir)?;

        // Create test config
        let config_content = r#"
nodes:
  test_node:
    test: true
    template: "test"
    priority: high
    reason: "Test node"
"#;
        fs::write(config_dir.join("node_config.yaml"), config_content)?;

        // Create test template
        let template_content = r#"
input_wrapper: "{input}"
cst_wrapper: "{fragment}"
metadata:
  nesting_level: 0
  example_input: "test_input"
  description: "Test template"
"#;
        fs::write(template_dir.join("test.yaml"), template_content)?;

        // Run scaffolder in dry-run mode
        let scaffolder = Scaffolder::new(
            config_dir.join("node_config.yaml"),
            template_dir,
            output_dir.clone(),
        );

        let report = scaffolder.scaffold(true)?;

        assert_eq!(report.total_nodes, 1);
        assert_eq!(report.generated, 1);
        assert_eq!(report.skipped, 0);
        assert!(report.errors.is_empty());

        // Verify output file was NOT created (dry run)
        let output_empty = if output_dir.exists() {
            let mut entries = fs::read_dir(&output_dir)?;
            entries.next().is_none()
        } else {
            true
        };
        assert!(output_empty);

        // Clean up
        let _ = fs::remove_dir_all(temp_dir);
        Ok(())
    }

    /// Tests scaffolder real run.
    #[test]
    fn test_scaffolder_real_run() -> Result<()> {
        // Create temporary config and templates
        let temp_dir = std::env::temp_dir().join("test_scaffold_real");
        let config_dir = temp_dir.join("config");
        let template_dir = temp_dir.join("templates");
        let output_dir = temp_dir.join("output");

        fs::create_dir_all(&config_dir)?;
        fs::create_dir_all(&template_dir)?;

        // Create test config
        let config_content = r#"
nodes:
  test_node:
    test: true
    template: "test"
    priority: high
    reason: "Test node"

  skipped_node:
    test: false
    template: null
    priority: low
    reason: "Skip this"
"#;
        fs::write(config_dir.join("node_config.yaml"), config_content)?;

        // Create test template
        let template_content = r#"
input_wrapper: "{input}"
cst_wrapper: "{fragment}"
metadata:
  nesting_level: 0
  example_input: "test_input"
  description: "Test template"
"#;
        fs::write(template_dir.join("test.yaml"), template_content)?;

        // Run scaffolder
        let scaffolder = Scaffolder::new(
            config_dir.join("node_config.yaml"),
            template_dir,
            output_dir.clone(),
        );

        let report = scaffolder.scaffold(false)?;

        assert_eq!(report.total_nodes, 2);
        assert_eq!(report.generated, 1); // Only test_node
        assert_eq!(report.skipped, 0);
        assert!(report.errors.is_empty());

        // Verify output file was created
        let spec_path = output_dir.join("test_node_example.md");
        assert!(spec_path.exists());

        let content = fs::read_to_string(&spec_path)?;
        assert!(content.contains("# test_node_example"));
        // Note: scaffold.rs now uses example library, so "test_node" gets "hello" (default)
        // instead of "test_input" from template metadata
        assert!(content.contains("hello")); // Default example for unknown nodes

        // Clean up
        let _ = fs::remove_dir_all(temp_dir);
        Ok(())
    }
}
