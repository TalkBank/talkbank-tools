//! Semantic tokens provider for syntax highlighting in LSP
//!
//! This module provides LSP semantic tokens that map tree-sitter highlight
//! tokens to LSP token types for syntax highlighting in editors.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

use crate::highlight::{HighlightConfig, TokenType};
use tower_lsp::lsp_types::*;

/// Semantic tokens provider
pub struct SemanticTokensProvider {
    config: HighlightConfig,
}

impl SemanticTokensProvider {
    /// Create a new semantic tokens provider.
    ///
    /// Builds the `HighlightConfig` so we can expose semantic tokens that match the manual’s documented
    /// token hierarchy via the `tree-sitter` capture sets.
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            config: HighlightConfig::new()?,
        })
    }

    /// Generate semantic tokens for the entire document.
    ///
    /// Converts the offset spans returned by `HighlightConfig` into delta-encoded LSP semantic tokens so
    /// editors receive the full CHAT token stream described in the File Format/Headers/Main Tier sections.
    pub fn semantic_tokens_full(&mut self, text: &str) -> Result<Vec<SemanticToken>, String> {
        let tokens = self.config.highlight(text)?;

        // Convert to LSP SemanticToken format (delta-encoded)
        let mut lsp_tokens = Vec::new();
        let mut prev_line = 0;
        let mut prev_char = 0;

        for token in tokens {
            let (line, char) = byte_offset_to_position(text, token.start);
            let length = (token.end - token.start) as u32;

            lsp_tokens.push(SemanticToken {
                delta_line: line - prev_line,
                delta_start: if line == prev_line {
                    char - prev_char
                } else {
                    char
                },
                length,
                token_type: Self::token_type_to_index(token.token_type),
                token_modifiers_bitset: 0,
            });

            prev_line = line;
            prev_char = char;
        }

        Ok(lsp_tokens)
    }

    /// Generate semantic tokens for a range of the document.
    ///
    /// Highlights the full document then filters to tokens overlapping the given
    /// byte range `[start_offset, end_offset)`. Delta encoding restarts from (0,0)
    /// for the first token in the range.
    pub fn semantic_tokens_range(
        &mut self,
        text: &str,
        start_offset: usize,
        end_offset: usize,
    ) -> Result<Vec<SemanticToken>, String> {
        let tokens = self.config.highlight(text)?;

        let mut lsp_tokens = Vec::new();
        let mut prev_line = 0;
        let mut prev_char = 0;
        let mut first = true;

        for token in tokens {
            // Skip tokens entirely before or after the range.
            if token.end <= start_offset || token.start >= end_offset {
                continue;
            }

            let (line, char) = byte_offset_to_position(text, token.start);
            let length = (token.end - token.start) as u32;

            if first {
                // First token in range: absolute position.
                lsp_tokens.push(SemanticToken {
                    delta_line: line,
                    delta_start: char,
                    length,
                    token_type: Self::token_type_to_index(token.token_type),
                    token_modifiers_bitset: 0,
                });
                first = false;
            } else {
                lsp_tokens.push(SemanticToken {
                    delta_line: line - prev_line,
                    delta_start: if line == prev_line {
                        char - prev_char
                    } else {
                        char
                    },
                    length,
                    token_type: Self::token_type_to_index(token.token_type),
                    token_modifiers_bitset: 0,
                });
            }

            prev_line = line;
            prev_char = char;
        }

        Ok(lsp_tokens)
    }

    /// Map a `TokenType` into the LSP semantic token legend index.
    ///
    /// The mapping keeps the LSP legend in sync with the manual’s categories (keywords, variables, strings,
    /// comments, etc.) and includes a few custom token types (tag/punctuation/error) for CHAT-specific symbols.
    fn token_type_to_index(token_type: TokenType) -> u32 {
        match token_type {
            TokenType::Keyword => 0,          // keyword
            TokenType::KeywordDirective => 0, // keyword (directives are also keywords)
            TokenType::Variable => 1,         // variable
            TokenType::String => 2,           // string
            TokenType::StringSpecial => 2,    // string (special strings still strings)
            TokenType::Comment => 3,          // comment
            TokenType::Type => 4,             // type
            TokenType::TypeBuiltin => 4,      // type
            TokenType::Operator => 5,         // operator
            TokenType::Number => 6,           // number
            TokenType::Function => 7,         // function
            TokenType::Tag => 8,              // custom: tag
            TokenType::Punctuation => 9,      // custom: punctuation
            TokenType::Error => 10,           // custom: error
        }
    }

    /// Get the semantic token legend for LSP initialization.
    ///
    /// The legend lists the token kinds used in `semantic_tokens_full` so the LSP client can colorize them
    /// consistent with the manual’s semantic layers.
    pub fn legend() -> SemanticTokensLegend {
        SemanticTokensLegend {
            token_types: vec![
                SemanticTokenType::KEYWORD,            // 0
                SemanticTokenType::VARIABLE,           // 1
                SemanticTokenType::STRING,             // 2
                SemanticTokenType::COMMENT,            // 3
                SemanticTokenType::TYPE,               // 4
                SemanticTokenType::OPERATOR,           // 5
                SemanticTokenType::NUMBER,             // 6
                SemanticTokenType::FUNCTION,           // 7
                SemanticTokenType::new("tag"),         // 8 (custom)
                SemanticTokenType::new("punctuation"), // 9 (custom)
                SemanticTokenType::new("error"),       // 10 (custom)
            ],
            token_modifiers: vec![],
        }
    }
}

/// Convert a byte offset to an (line, character) position for LSP API consumption.
///
/// Allows the semantic tokens provider to translate capture ranges (byte-based) into the line/column
/// coordinates mandated by the LSP protocol and described in the CHAT file layout.
fn byte_offset_to_position(text: &str, offset: usize) -> (u32, u32) {
    let mut line = 0;
    let mut line_start = 0;

    for (idx, ch) in text.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = idx + 1;
        }
    }

    (line, (offset - line_start) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests semantic tokens provider creation.
    #[test]
    fn test_semantic_tokens_provider_creation() {
        let result = SemanticTokensProvider::new();
        assert!(
            result.is_ok(),
            "Failed to create SemanticTokensProvider: {:?}",
            result.err()
        );
    }

    /// Tests semantic tokens generation.
    #[test]
    fn test_semantic_tokens_generation() -> Result<(), String> {
        let mut provider = SemanticTokensProvider::new()
            .map_err(|err| format!("Failed to create SemanticTokensProvider: {err}"))?;

        let text = "@UTF8\n@Begin\n*CHI:\thello .\n@End\n";
        let tokens = provider
            .semantic_tokens_full(text)
            .map_err(|err| format!("Failed to generate semantic tokens: {err}"))?;

        // Should have some tokens
        assert!(!tokens.is_empty(), "No semantic tokens generated");
        Ok(())
    }

    #[test]
    fn test_semantic_tokens_range_subset() -> Result<(), String> {
        let mut provider = SemanticTokensProvider::new()
            .map_err(|err| format!("Failed to create SemanticTokensProvider: {err}"))?;

        let text = "@UTF8\n@Begin\n*CHI:\thello .\n@End\n";
        let full = provider
            .semantic_tokens_full(text)
            .map_err(|err| format!("Full tokens error: {err}"))?;

        // Range covering only the last line "@End\n" (offset 27..32)
        let range = provider
            .semantic_tokens_range(text, 27, 32)
            .map_err(|err| format!("Range tokens error: {err}"))?;

        // Range should have fewer tokens than full.
        assert!(
            range.len() < full.len(),
            "Range should be a subset of full tokens"
        );
        assert!(
            !range.is_empty(),
            "Range should have at least one token for @End"
        );
        Ok(())
    }

    /// Tests byte offset to position.
    #[test]
    fn test_byte_offset_to_position() {
        let text = "hello\nworld";
        assert_eq!(byte_offset_to_position(text, 0), (0, 0));
        assert_eq!(byte_offset_to_position(text, 5), (0, 5)); // at newline
        assert_eq!(byte_offset_to_position(text, 6), (1, 0)); // start of line 2
        assert_eq!(byte_offset_to_position(text, 10), (1, 4)); // 'l' in world
    }
}
