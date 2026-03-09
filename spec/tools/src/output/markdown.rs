//! # Markdown Documentation Generator
//!
//! Generates publishable error documentation in Markdown format

use crate::spec::error::{ErrorDefinition, ErrorSpec};

/// Generate a Markdown page for a single error
pub fn generate_error_page(error: &ErrorDefinition) -> String {
    let mut output = String::new();

    // Title and metadata
    output.push_str(&format!("# {}: {}\n\n", error.code, error.name));
    output.push_str(&format!("**Severity**: {}\n\n", error.severity));

    // Description
    output.push_str("## Description\n\n");
    output.push_str(&format!("{}\n\n", error.description));

    // Examples
    if !error.examples.is_empty() {
        output.push_str("## Examples\n\n");
        for (i, example) in error.examples.iter().enumerate() {
            output.push_str(&format!("### Example {}\n\n", i + 1));
            output.push_str("```chat\n");
            output.push_str(&example.input);
            output.push_str("\n```\n\n");
            output.push_str(&format!("**Error**: {}\n\n", example.expected_message));
        }
    }

    // How to fix
    output.push_str("## How to Fix\n\n");
    output.push_str(&format!("{}\n\n", error.suggestion));

    // Help URL
    if let Some(url) = &error.help_url {
        output.push_str("## More Information\n\n");
        output.push_str(&format!("[CHAT Manual]({})\n\n", url));
    }

    output
}

/// Generate index page for all errors
pub fn generate_error_index(specs: &[ErrorSpec]) -> String {
    let mut output = String::new();

    output.push_str("# CHAT Error Reference\n\n");
    output.push_str("Complete reference for all CHAT parser and validation errors.\n\n");

    // Group by category
    for spec in specs {
        output.push_str(&format!(
            "## {} ({})\n\n",
            spec.metadata.category, spec.metadata.range
        ));
        output.push_str(&format!("{}\n\n", spec.metadata.description));

        output.push_str("| Code | Name | Severity |\n");
        output.push_str("|------|------|----------|\n");

        for error in &spec.errors {
            output.push_str(&format!(
                "| [{}]({}.md) | {} | {} |\n",
                error.code, error.code, error.name, error.severity
            ));
        }

        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::error::*;

    /// Tests generate error page.
    #[test]
    fn test_generate_error_page() {
        let error = ErrorDefinition {
            code: "E241".to_string(),
            name: "IllegalUntranscribed".to_string(),
            severity: "error".to_string(),
            description: "Word contains illegal untranscribed marker".to_string(),
            suggestion: "Use 'xxx' for unintelligible speech".to_string(),
            help_url: Some("https://talkbank.org/errors/E241".to_string()),
            references: ErrorReference::default(),
            examples: vec![],
        };

        let output = generate_error_page(&error);
        assert!(output.contains("# E241"));
        assert!(output.contains("IllegalUntranscribed"));
        assert!(output.contains("**Severity**: error"));
    }
}
