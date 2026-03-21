//! YAML-based template system for wrapping node fragments into complete spec files.
//!
//! Each template defines an `input_wrapper` (CHAT source with a `{input}`
//! placeholder) and a `cst_wrapper` (expected CST with a `{fragment}`
//! placeholder).  Templates support single-inheritance via an `extends` field:
//! a child template can override any field, and unset fields fall through to the
//! resolved parent.
//!
//! [`TemplateLoader`] provides a caching layer that loads templates by name,
//! resolves inheritance chains, and returns references to the fully-resolved
//! result.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur while loading or resolving templates.
#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Failed to read template file: {path}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to parse template YAML: {path}")]
    Parse {
        path: PathBuf,
        source: serde_yaml_ng::Error,
    },
    #[error("Template not found in cache: {name}")]
    MissingCache { name: String },
}

/// A scaffold template that wraps a node-specific fragment into a complete CHAT
/// input and its expected CST output.
///
/// Templates are loaded from YAML files in the template directory and may form
/// single-inheritance chains via [`extends`](Self::extends).  After resolution,
/// [`apply`](Self::apply) performs variable substitution on both wrappers.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Template {
    /// Name of a parent template (without `.yaml` extension) whose fields
    /// should be inherited.  After [`resolve`](Self::resolve), this is always
    /// `None`.  Circular chains are not detected and will stack-overflow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,

    /// CHAT source text with a `{input}` placeholder that will be replaced by
    /// the node-specific example input.  Empty string means "inherit from
    /// parent".
    #[serde(default)]
    pub input_wrapper: String,

    /// Expected CST text with a `{fragment}` placeholder that will be replaced
    /// by the indented node CST.  Empty string means "inherit from parent".
    #[serde(default)]
    pub cst_wrapper: String,

    /// Ancillary information about indentation depth, example input, and
    /// human-readable purpose.
    #[serde(default)]
    pub metadata: TemplateMetadata,
}

/// Ancillary metadata attached to a [`Template`].
///
/// All fields default to their zero/empty value, which signals "inherit from
/// parent" during template resolution.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TemplateMetadata {
    /// Number of leading spaces to prepend to each line of the `{fragment}`
    /// CST before substitution.  A value of `0` means no indentation (and
    /// also means "inherit from parent" during resolution).
    #[serde(default)]
    pub nesting_level: usize,

    /// A representative CHAT input string for this template, used as the
    /// default when no node-specific example exists in the example library.
    #[serde(default)]
    pub example_input: String,

    /// Human-readable description of the template's purpose, carried through
    /// into the generated spec markdown.
    #[serde(default)]
    pub description: String,
}

impl Template {
    /// Load template from YAML file
    ///
    /// # Arguments
    /// * `path` - Path to template YAML file
    ///
    /// # Returns
    /// * `Ok(Template)` - Loaded template (may have unresolved inheritance)
    /// * `Err(String)` - Error message if loading fails
    pub fn load(path: &Path) -> Result<Self, TemplateError> {
        let content = fs::read_to_string(path).map_err(|source| TemplateError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        let template: Template =
            serde_yaml_ng::from_str(&content).map_err(|source| TemplateError::Parse {
                path: path.to_path_buf(),
                source,
            })?;

        Ok(template)
    }

    /// Resolve template inheritance
    ///
    /// If this template extends another, loads the parent and merges.
    /// Child template fields override parent fields.
    ///
    /// # Arguments
    /// * `template_dir` - Directory containing template files
    ///
    /// # Returns
    /// * `Ok(Template)` - Fully resolved template
    /// * `Err(String)` - Error if parent template not found or circular dependency
    pub fn resolve(&self, template_dir: &Path) -> Result<Self, TemplateError> {
        if let Some(parent_name) = &self.extends {
            // Load parent template
            let parent_path = template_dir.join(format!("{}.yaml", parent_name));
            let parent = Template::load(&parent_path)?;

            // Recursively resolve parent (handles multi-level inheritance)
            let resolved_parent = parent.resolve(template_dir)?;

            // Merge: child overrides parent
            Ok(Self {
                extends: None, // Clear extends after resolving
                input_wrapper: if self.input_wrapper.is_empty() {
                    resolved_parent.input_wrapper
                } else {
                    self.input_wrapper.clone()
                },
                cst_wrapper: if self.cst_wrapper.is_empty() {
                    resolved_parent.cst_wrapper
                } else {
                    self.cst_wrapper.clone()
                },
                metadata: TemplateMetadata {
                    nesting_level: if self.metadata.nesting_level == 0 {
                        resolved_parent.metadata.nesting_level
                    } else {
                        self.metadata.nesting_level
                    },
                    example_input: if self.metadata.example_input.is_empty() {
                        resolved_parent.metadata.example_input
                    } else {
                        self.metadata.example_input.clone()
                    },
                    description: if self.metadata.description.is_empty() {
                        resolved_parent.metadata.description
                    } else {
                        self.metadata.description.clone()
                    },
                },
            })
        } else {
            // No parent, return self
            Ok(self.clone())
        }
    }

    /// Apply template to generate input and CST
    ///
    /// # Arguments
    /// * `input` - The example input text
    /// * `fragment_cst` - The CST fragment (pre-formatted, not indented)
    ///
    /// # Returns
    /// * `(String, String)` - (wrapped_input, wrapped_cst)
    pub fn apply(&self, input: &str, fragment_cst: &str) -> (String, String) {
        // Apply input wrapper
        let wrapped_input = self.input_wrapper.replace("{input}", input);

        // Indent fragment CST
        let indented_fragment = indent_text(fragment_cst, self.metadata.nesting_level);

        // Apply CST wrapper
        let wrapped_cst = self.cst_wrapper.replace("{fragment}", &indented_fragment);

        (wrapped_input, wrapped_cst)
    }
}

/// Indent text by a given number of spaces
///
/// # Arguments
/// * `text` - Text to indent
/// * `spaces` - Number of spaces to indent each line
///
/// # Returns
/// * Indented text
fn indent_text(text: &str, spaces: usize) -> String {
    if spaces == 0 {
        return text.to_string();
    }

    let indent = " ".repeat(spaces);
    text.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{}{}", indent, line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Loads templates by name from a directory, resolves inheritance, and caches
/// the result so repeated lookups for the same template are free.
pub struct TemplateLoader {
    template_dir: PathBuf,
    cache: HashMap<String, Template>,
}

impl TemplateLoader {
    /// Create new template loader
    ///
    /// # Arguments
    /// * `template_dir` - Directory containing template YAML files
    pub fn new(template_dir: PathBuf) -> Self {
        Self {
            template_dir,
            cache: HashMap::new(),
        }
    }

    /// Load and resolve template by name
    ///
    /// # Arguments
    /// * `name` - Template name (without .yaml extension)
    ///
    /// # Returns
    /// * `Ok(&Template)` - Cached, fully-resolved template
    /// * `Err(String)` - Error if template not found or invalid
    pub fn load(&mut self, name: &str) -> Result<&Template, TemplateError> {
        // Check cache first
        if !self.cache.contains_key(name) {
            // Load from file
            let template_path = self.template_dir.join(format!("{}.yaml", name));
            let template = Template::load(&template_path)?;

            // Resolve inheritance
            let resolved = template.resolve(&self.template_dir)?;

            // Cache
            self.cache.insert(name.to_string(), resolved);
        }

        self.cache
            .get(name)
            .ok_or_else(|| TemplateError::MissingCache {
                name: name.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    /// Tests indent text.
    #[test]
    fn test_indent_text() -> Result<()> {
        let text = "line1\nline2\nline3";
        let indented = indent_text(text, 4);
        assert_eq!(indented, "    line1\n    line2\n    line3");

        let text_with_empty = "line1\n\nline3";
        let indented = indent_text(text_with_empty, 2);
        assert_eq!(indented, "  line1\n\n  line3");

        let no_indent = indent_text("test", 0);
        assert_eq!(no_indent, "test");
        Ok(())
    }

    /// Tests template apply.
    #[test]
    fn test_template_apply() -> Result<()> {
        let template = Template {
            extends: None,
            input_wrapper: "Input: {input}".to_string(),
            cst_wrapper: "CST:\n{fragment}".to_string(),
            metadata: TemplateMetadata {
                nesting_level: 2,
                example_input: "example".to_string(),
                description: "Test template".to_string(),
            },
        };

        let (input, cst) = template.apply("hello", "node1\nnode2");
        assert_eq!(input, "Input: hello");
        assert_eq!(cst, "CST:\n  node1\n  node2");
        Ok(())
    }

    /// Tests template load and resolve.
    #[test]
    fn test_template_load_and_resolve() -> Result<()> {
        // Create temporary templates
        let temp_dir = std::env::temp_dir().join("test_templates");
        fs::create_dir_all(&temp_dir)?;

        // Base template
        let base_content = r#"
input_wrapper: |
  BASE INPUT: {input}

cst_wrapper: |
  BASE CST:
  {fragment}

metadata:
  nesting_level: 5
  example_input: "base_example"
  description: "Base template"
"#;
        fs::write(temp_dir.join("_base.yaml"), base_content)?;

        // Child template extending base
        let child_content = r#"
extends: _base

metadata:
  nesting_level: 10
  description: "Child template"
"#;
        fs::write(temp_dir.join("child.yaml"), child_content)?;

        // Load and resolve child
        let child = Template::load(&temp_dir.join("child.yaml"))?;
        let resolved = child.resolve(&temp_dir)?;

        // Verify inheritance
        assert_eq!(resolved.extends, None);
        assert!(resolved.input_wrapper.contains("BASE INPUT"));
        assert!(resolved.cst_wrapper.contains("BASE CST"));
        assert_eq!(resolved.metadata.nesting_level, 10); // Child override
        assert_eq!(resolved.metadata.example_input, "base_example"); // From parent
        assert_eq!(resolved.metadata.description, "Child template"); // Child override

        // Clean up
        let _ = fs::remove_dir_all(temp_dir);
        Ok(())
    }

    /// Tests template loader.
    #[test]
    fn test_template_loader() -> Result<()> {
        // Create temporary templates
        let temp_dir = std::env::temp_dir().join("test_loader_templates");
        fs::create_dir_all(&temp_dir)?;

        let template_content = r#"
input_wrapper: "{input}"
cst_wrapper: "{fragment}"
metadata:
  nesting_level: 0
  example_input: "test"
  description: "Test"
"#;
        fs::write(temp_dir.join("test.yaml"), template_content)?;

        let mut loader = TemplateLoader::new(temp_dir.clone());

        // Load template
        let template = loader.load("test")?;
        assert_eq!(template.metadata.example_input, "test");

        // Load again (should use cache)
        let template2 = loader.load("test")?;
        assert_eq!(template2.metadata.example_input, "test");

        // Clean up
        let _ = fs::remove_dir_all(temp_dir);
        Ok(())
    }
}
