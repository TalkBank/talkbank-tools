//! Parses `@Comment:` headers in CHAT fixture files to extract the
//! `extract=<node_name>` directive that tells the CST extractor which
//! tree-sitter node type to target.
//!
//! Fixture files embed their target node in a comment so the same `.cha` file
//! serves as both a valid CHAT document and a self-describing test input.

use chumsky::{error::Simple, prelude::*};
use talkbank_model::model::Header;
use talkbank_parser::TreeSitterParser;
use thiserror::Error;

/// Errors that can occur while parsing an extract directive from a fixture.
#[derive(Debug, Error)]
pub enum FixtureError {
    /// The CHAT content could not be parsed at all (e.g. the tree-sitter parser
    /// could not be initialized or the file is fundamentally malformed).
    #[error("Failed to parse CHAT fixture")]
    ParseFailed,
    /// The file parsed successfully but contains no `@Comment:` header, so
    /// there is nowhere to look for an extract directive.
    #[error("No @Comment header found in fixture")]
    NoCommentHeader,
    /// A `@Comment:` header exists but its content does not contain the word
    /// `extract`, so no directive was found.
    #[error("@Comment header missing extract directive")]
    NoExtractDirective,
    /// The comment contains `extract` but the directive does not match the
    /// expected `extract=<identifier>` format.  The string payload describes
    /// the expected format.
    #[error("Invalid extract directive format: {0}")]
    InvalidDirective(String),
}

/// Extracts directive parser.
fn extract_directive_parser<'src>(
) -> impl Parser<'src, &'src str, String, extra::Err<Simple<'src, char>>> {
    let ident = any::<_, extra::Err<Simple<'src, char>>>()
        .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_')
        .repeated()
        .at_least(1)
        .collect::<String>();

    text::keyword("extract")
        .padded()
        .ignore_then(just('=').padded())
        .ignore_then(ident)
        .then_ignore(end())
}

/// Parses the extract directive from a @Comment header in a CHAT fixture file.
///
/// Expected format: `@Comment:\textract=<node_name>`
///
/// # Arguments
/// * `source` - The full CHAT file content
///
/// # Returns
/// * `Ok(String)` - The extracted node name
/// * `Err(FixtureError)` - If parsing fails
pub fn parse_extract_directive(source: &str) -> Result<String, FixtureError> {
    let parser = TreeSitterParser::new().map_err(|_| FixtureError::ParseFailed)?;
    let chat_file = parser
        .parse_chat_file(source)
        .map_err(|_| FixtureError::ParseFailed)?;

    let mut comment_content = None;
    for line in chat_file.lines.iter() {
        if let Some(Header::Comment { content }) = line.as_header() {
            comment_content = Some(content.to_chat_string());
            break;
        }
    }

    let content = match comment_content {
        Some(content) => content,
        None => return Err(FixtureError::NoCommentHeader),
    };

    let directive = extract_directive_parser()
        .parse(content.as_str())
        .into_result();
    match directive {
        Ok(node_name) => Ok(node_name),
        Err(_) => {
            if content.contains("extract") {
                Err(FixtureError::InvalidDirective(
                    "extract directive must be in format 'extract=<node_name>'".to_string(),
                ))
            } else {
                Err(FixtureError::NoExtractDirective)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    /// Tests valid extract directive.
    #[test]
    fn test_valid_extract_directive() -> Result<()> {
        let content = "@UTF8\n@Begin\n@Comment:\textract=word\n*CHI:\thello .\n@End";
        let result = parse_extract_directive(content)?;
        assert_eq!(result, "word");
        Ok(())
    }

    /// Tests extract with spaces.
    #[test]
    fn test_extract_with_spaces() -> Result<()> {
        let content = "@UTF8\n@Begin\n@Comment:\textract=word_with_suffix\n*CHI:\ttest .\n@End";
        let result = parse_extract_directive(content)?;
        assert_eq!(result, "word_with_suffix");
        Ok(())
    }

    /// Tests no comment header.
    #[test]
    fn test_no_comment_header() -> Result<()> {
        let content = "@UTF8\n@Begin\n*CHI:\thello .\n@End";
        let result = parse_extract_directive(content);
        assert!(matches!(result, Err(FixtureError::NoCommentHeader)));
        Ok(())
    }

    /// Tests comment without extract.
    #[test]
    fn test_comment_without_extract() -> Result<()> {
        let content = "@UTF8\n@Begin\n@Comment:\tsome other comment\n*CHI:\thello .\n@End";
        let result = parse_extract_directive(content);
        assert!(matches!(result, Err(FixtureError::NoExtractDirective)));
        Ok(())
    }

    /// Tests invalid extract format.
    #[test]
    fn test_invalid_extract_format() -> Result<()> {
        let content = "@UTF8\n@Begin\n@Comment:\textract\n*CHI:\thello .\n@End";
        let result = parse_extract_directive(content);
        assert!(matches!(result, Err(FixtureError::InvalidDirective(_))));
        Ok(())
    }
}
