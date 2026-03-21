//! Extracts named node definitions from `grammar.js` using the oxc JavaScript
//! parser.
//!
//! The bootstrap pipeline needs a list of every named rule in the tree-sitter
//! grammar so it can classify and scaffold spec tests.  This module parses
//! `grammar.js`, locates the `rules` object inside `export default grammar({...})`,
//! and returns a [`NodeInfo`] for each non-anonymous rule (rules whose names do
//! not start with `_`).

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// A named rule extracted from `grammar.js`.
///
/// Each instance represents one public (non-anonymous) tree-sitter node type
/// that could appear in a parsed CST.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// The rule name as written in `grammar.js` (e.g. `"standalone_word"`).
    /// Anonymous helper rules (names starting with `_`) are filtered out during
    /// extraction and will never appear here.
    pub name: String,
    /// The raw JavaScript source text of the rule's value expression, extracted
    /// by byte span from `grammar.js`.  Useful for heuristic analysis of rule
    /// complexity but not parsed further.
    pub rule_definition: String,
}

/// Errors that can occur while extracting nodes from `grammar.js`.
#[derive(Debug, Error)]
pub enum GrammarError {
    /// The grammar file could not be read from disk (missing, permissions, etc.).
    #[error("Failed to read grammar file: {path}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    /// The oxc JavaScript parser reported syntax errors in `grammar.js`.
    #[error("Failed to parse grammar.js with oxc")]
    Parse,
    /// The file parsed successfully but does not contain the expected
    /// `export default grammar({...})` top-level call.
    #[error("Could not find export default grammar(...) call")]
    MissingGrammarCall,
    /// The grammar object was found but it has no `rules` property, or the
    /// `rules` property contains zero named (non-anonymous) entries.
    #[error("Could not find rules object in grammar")]
    MissingRules,
    /// A rule property exists but its key could not be resolved to a string
    /// name (e.g. a computed property key).
    #[error("Rule property missing or invalid")]
    InvalidRule,
}

/// Extract all named nodes from grammar.js
///
/// Parses the JavaScript grammar file and extracts all rule definitions.
/// Only returns named nodes (not starting with underscore).
///
/// # Arguments
/// * `grammar_path` - Path to grammar.js file
///
/// # Returns
/// * `Ok(Vec<NodeInfo>)` - List of all named nodes with their definitions
/// * `Err(GrammarError)` - Error if parsing fails
pub fn extract_nodes(grammar_path: &Path) -> Result<Vec<NodeInfo>, GrammarError> {
    let content = fs::read_to_string(grammar_path).map_err(|source| GrammarError::Read {
        path: grammar_path.display().to_string(),
        source,
    })?;

    let allocator = Allocator::default();
    let source_type = SourceType::default().with_module(true);
    let ret = Parser::new(&allocator, &content, source_type).parse();
    if !ret.errors.is_empty() {
        return Err(GrammarError::Parse);
    }

    let program = ret.program;
    let grammar_object = find_grammar_object(&program).ok_or(GrammarError::MissingGrammarCall)?;
    let rules_object = find_rules_object(grammar_object).ok_or(GrammarError::MissingRules)?;

    let mut nodes = Vec::new();
    for property in &rules_object.properties {
        let ObjectPropertyKind::ObjectProperty(prop) = property else {
            continue;
        };

        let rule_name = property_key_name(&prop.key).ok_or(GrammarError::InvalidRule)?;
        if is_anonymous_rule(rule_name.as_str()) {
            continue;
        }

        let rule_definition = slice_span(&content, prop.value.span());
        nodes.push(NodeInfo {
            name: rule_name,
            rule_definition,
        });
    }

    if nodes.is_empty() {
        return Err(GrammarError::MissingRules);
    }

    Ok(nodes)
}

/// Returns `true` if the rule name starts with `_`, marking it as a tree-sitter
/// anonymous (hidden) rule that should not appear as a named node in the CST.
fn is_anonymous_rule(name: &str) -> bool {
    name.as_bytes().first().is_some_and(|b| *b == b'_')
}

/// Locates the object literal passed to `grammar(...)` in the program's
/// `export default` declaration.
fn find_grammar_object<'a>(program: &'a Program<'a>) -> Option<&'a ObjectExpression<'a>> {
    for statement in &program.body {
        let Statement::ExportDefaultDeclaration(export) = statement else {
            continue;
        };

        let ExportDefaultDeclarationKind::CallExpression(call) = &export.declaration else {
            continue;
        };

        if !is_grammar_call(call) {
            continue;
        }

        let arg = call.arguments.first()?;
        let Argument::ObjectExpression(obj) = arg else {
            continue;
        };

        return Some(obj);
    }
    None
}

/// Returns `true` if the call expression invokes the `grammar` function by name.
fn is_grammar_call(call: &CallExpression) -> bool {
    match &call.callee {
        Expression::Identifier(ident) => ident.name.as_str() == "grammar",
        _ => false,
    }
}

/// Finds the `rules` property inside the grammar object and returns its value
/// as an [`ObjectExpression`].
fn find_rules_object<'a>(
    grammar_obj: &'a ObjectExpression<'a>,
) -> Option<&'a ObjectExpression<'a>> {
    let rule_prop = grammar_obj
        .properties
        .iter()
        .filter_map(|property| match property {
            ObjectPropertyKind::ObjectProperty(prop) => Some(prop.as_ref()),
            _ => None,
        })
        .find(|prop| property_key_is(&prop.key, "rules"))?;

    match &rule_prop.value {
        Expression::ObjectExpression(obj) => Some(obj.as_ref()),
        _ => None,
    }
}

/// Returns `true` if the property key resolves to the string `expected`.
fn property_key_is(key: &PropertyKey, expected: &str) -> bool {
    property_key_name(key).is_some_and(|name| name.as_str() == expected)
}

/// Extracts the string name from a property key, supporting both identifiers
/// and string literals.  Returns `None` for computed keys.
fn property_key_name(key: &PropertyKey) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(ident) => Some(ident.name.as_str().to_string()),
        PropertyKey::StringLiteral(lit) => Some(lit.value.as_str().to_string()),
        _ => None,
    }
}

/// Extracts a substring from `source` using an oxc byte span.  Returns an
/// empty string if the span is out of bounds.
fn slice_span(source: &str, span: oxc_span::Span) -> String {
    let start = span.start as usize;
    let end = span.end as usize;
    if start >= end || end > source.len() {
        return String::new();
    }
    source[start..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use talkbank_parser::node_types;

    /// Tests extract nodes from real grammar.
    #[test]
    fn test_extract_nodes_from_real_grammar() -> Result<()> {
        let grammar_path = Path::new(env!("HOME")).join("projects/tree-sitter-talkbank/grammar.js");

        if !grammar_path.exists() {
            eprintln!("Skipping test - grammar.js not found at {:?}", grammar_path);
            return Ok(());
        }

        let nodes = extract_nodes(&grammar_path)?;

        // Should find ~500 named nodes
        assert!(
            nodes.len() > 400,
            "Expected ~500 nodes, got {}",
            nodes.len()
        );
        assert!(
            nodes.len() < 600,
            "Expected ~500 nodes, got {}",
            nodes.len()
        );

        // Verify key nodes are present
        let node_names: Vec<&str> = nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(node_names.contains(&node_types::DOCUMENT));
        assert!(node_names.contains(&node_types::LINE));
        assert!(node_names.contains(&node_types::STANDALONE_WORD));
        assert!(node_names.contains(&node_types::MOR_DEPENDENT_TIER));

        // Verify no anonymous nodes (starting with _)
        for node in &nodes {
            assert!(
                !is_anonymous_rule(&node.name),
                "Found anonymous node: {}",
                node.name
            );
        }

        Ok(())
    }
}
