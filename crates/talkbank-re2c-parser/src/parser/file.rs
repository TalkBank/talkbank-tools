//! File-level parser — assembles headers, utterances, and dependent tiers.
//!
//! This module is imperative (sequential line dispatch) rather than
//! combinator-based, because the file structure is prefix-dispatched:
//! dependent tier type is determined by reading the `%mor:`, `%gra:` etc.
//! prefix text, which doesn't map cleanly to chumsky's token-variant matching.
//!
//! Sub-parsers for individual tiers are chumsky combinators from
//! `dependent_tiers` and `main_tier`.

use chumsky::Parser as _;

use crate::ast::*;
use crate::token::{Token, TokenDiscriminants};
use talkbank_model::{ErrorSink, NullErrorSink, ParseError, Span};

use super::dependent_tiers;
use super::main_tier;

/// Parse a complete CHAT file with no error reporting.
pub fn parse_file<'a>(tokens: &'a [Token<'a>], source: &'a str) -> ChatFile<'a> {
    parse_file_with_errors(tokens, source, &NullErrorSink)
}

/// Parse a complete CHAT file from a leaked token slice, reporting
/// unhandled tokens to the given error sink.
pub fn parse_file_with_errors<'a>(
    tokens: &'a [Token<'a>],
    source: &'a str,
    errors: &impl ErrorSink,
) -> ChatFile<'a> {
    let mut pos = 0;
    let mut lines = Vec::new();

    while pos < tokens.len() {
        let d = TokenDiscriminants::from(&tokens[pos]);
        match d {
            // No-content headers
            TokenDiscriminants::HeaderUtf8
            | TokenDiscriminants::HeaderBegin
            | TokenDiscriminants::HeaderEnd
            | TokenDiscriminants::HeaderBlank
            | TokenDiscriminants::HeaderNewEpisode => {
                let tok = tokens[pos].clone();
                pos += 1;
                if pos < tokens.len()
                    && TokenDiscriminants::from(&tokens[pos]) == TokenDiscriminants::Newline
                {
                    pos += 1;
                }
                lines.push(Line::Header(HeaderParsed {
                    prefix: tok,
                    content: vec![],
                }));
            }

            // Headers with content
            TokenDiscriminants::HeaderPrefix
            | TokenDiscriminants::HeaderBirthOf
            | TokenDiscriminants::HeaderBirthplaceOf
            | TokenDiscriminants::HeaderL1Of => {
                let prefix = tokens[pos].clone();
                pos += 1;
                let mut content = Vec::new();
                while pos < tokens.len()
                    && TokenDiscriminants::from(&tokens[pos]) != TokenDiscriminants::Newline
                {
                    let tok = tokens[pos].clone();
                    pos += 1;
                    // Skip whitespace (structural) but keep continuations
                    if !matches!(tok, Token::Whitespace(_)) {
                        content.push(tok);
                    }
                }
                if pos < tokens.len()
                    && TokenDiscriminants::from(&tokens[pos]) == TokenDiscriminants::Newline
                {
                    pos += 1;
                }
                lines.push(Line::Header(HeaderParsed { prefix, content }));
            }

            // Main tier
            TokenDiscriminants::Star => {
                // Find the extent of the main tier line (up to and including newline)
                let start = pos;
                pos = skip_to_newline(tokens, pos);
                if pos < tokens.len() {
                    pos += 1; // consume newline
                }

                let main_tier_tokens = &tokens[start..pos];
                if let Ok(main_tier) =
                    main_tier::main_tier_parser().parse(main_tier_tokens).into_result()
                {
                    // Collect dependent tiers
                    let dep_tiers = parse_dependent_tiers(tokens, &mut pos);
                    lines.push(Line::Utterance(Utterance {
                        main_tier,
                        dependent_tiers: dep_tiers,
                    }));
                }
            }

            // Skip structural tokens
            TokenDiscriminants::Whitespace
            | TokenDiscriminants::Newline
            | TokenDiscriminants::Continuation
            | TokenDiscriminants::BOM => {
                pos += 1;
            }

            // Orphan tier prefix (no preceding main tier) — skip line
            TokenDiscriminants::TierPrefix => {
                pos = skip_to_newline(tokens, pos);
                if pos < tokens.len() {
                    pos += 1;
                }
            }

            // Unknown tokens — report and skip
            _ => {
                let tok = &tokens[pos];
                errors.report(ParseError::new(
                    talkbank_model::errors::codes::ErrorCode::UnexpectedSyntax,
                    talkbank_model::Severity::Warning,
                    talkbank_model::SourceLocation::new(Span::DUMMY),
                    None,
                    format!("unhandled token in parse_chat_file: {:?}", tok.text()),
                ));
                pos += 1;
            }
        }
    }

    ChatFile { lines, source }
}

/// Parse dependent tiers following a main tier.
///
/// Consumes `TierPrefix` lines from the token stream, dispatching each
/// to the appropriate chumsky sub-parser based on the prefix text.
fn parse_dependent_tiers<'a>(tokens: &'a [Token<'a>], pos: &mut usize) -> Vec<DependentTierParsed<'a>> {
    let mut dep_tiers = Vec::new();

    while *pos < tokens.len()
        && TokenDiscriminants::from(&tokens[*pos]) == TokenDiscriminants::TierPrefix
    {
        let prefix = tokens[*pos].clone();
        let prefix_text = prefix.text();
        *pos += 1;

        // Collect tier content tokens until newline
        let content_start = *pos;
        *pos = skip_to_newline(tokens, *pos);
        let content_end = *pos;
        if *pos < tokens.len() {
            *pos += 1; // consume newline
        }

        let tier_tokens = &tokens[content_start..content_end];

        if prefix_text.starts_with("%mor") || prefix_text.starts_with("%trn") {
            if let Ok(tier) = dependent_tiers::mor_tier_parser()
                .parse(tier_tokens)
                .into_result()
            {
                dep_tiers.push(DependentTierParsed::Mor(tier));
            }
        } else if prefix_text.starts_with("%pho") {
            if let Ok(tier) = dependent_tiers::pho_tier_parser()
                .parse(tier_tokens)
                .into_result()
            {
                dep_tiers.push(DependentTierParsed::Pho(tier));
            }
        } else if prefix_text.starts_with("%mod") {
            if let Ok(tier) = dependent_tiers::pho_tier_parser()
                .parse(tier_tokens)
                .into_result()
            {
                dep_tiers.push(DependentTierParsed::Mod(tier));
            }
        } else if prefix_text.starts_with("%gra") {
            if let Ok(tier) = dependent_tiers::gra_tier_parser()
                .parse(tier_tokens)
                .into_result()
            {
                dep_tiers.push(DependentTierParsed::Gra(tier));
            }
        } else if prefix_text.starts_with("%sin") {
            if let Ok(tier) = dependent_tiers::sin_tier_parser()
                .parse(tier_tokens)
                .into_result()
            {
                dep_tiers.push(DependentTierParsed::Sin(tier));
            }
        } else if prefix_text.starts_with("%wor") {
            if let Ok((items, terminator)) = dependent_tiers::wor_tier_parser()
                .parse(tier_tokens)
                .into_result()
            {
                dep_tiers.push(DependentTierParsed::Wor { items, terminator });
            }
        } else {
            // Generic text tier — collect content tokens
            let content: Vec<Token<'a>> = tier_tokens.to_vec();
            dep_tiers.push(DependentTierParsed::Text { prefix, content });
        }
    }

    dep_tiers
}

/// Advance position to the Newline token (or end of tokens).
fn skip_to_newline(tokens: &[Token<'_>], mut pos: usize) -> usize {
    while pos < tokens.len()
        && TokenDiscriminants::from(&tokens[pos]) != TokenDiscriminants::Newline
    {
        pos += 1;
    }
    pos
}
