#![warn(missing_docs)]
//! Direct parser for CHAT constructs in isolation.
//!
//! Unlike the tree-sitter parser (which parses complete CHAT transcripts top-to-bottom),
//! the direct parser is designed to parse **individual CHAT constructs** — a single word,
//! tier, header, or utterance — without requiring a full file context. This makes it the
//! right choice for downstream crates that need to parse or validate CHAT fragments
//! (e.g. a %mor line from UD output, or a main tier synthesized from ASR).
//!
//! All tier-parsing modules are public. Tier content parsers expect input **without** the
//! `%tier:\t` prefix (the [`ChatParser`] content-only convention).
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//!
//! # Examples
//!
//! ```
//! use talkbank_direct_parser::DirectParser;
//! use talkbank_model::{ChatParser, ParseOutcome};
//! use talkbank_model::ErrorCollector;
//!
//! let parser = DirectParser::new().unwrap();
//! let errors = ErrorCollector::new();
//! let word = parser.parse_word("hello", 0, &errors);
//! assert!(word.is_parsed());
//! ```

mod dependent_tier;
mod file;
pub mod gra_tier;
mod header;
pub mod main_tier;
pub mod mor_tier;
pub mod pho_tier;
mod recovery;
pub mod sin_tier;
pub mod text_tier;
pub mod tokens;
mod whitespace;
pub mod wor_tier;
pub mod word;

use talkbank_model::dependent_tier::DependentTier;
use talkbank_model::model::{
    ActTier, AddTier, ChatFile, CodTier, ComTier, ExpTier, GpxTier, GraTier, GrammaticalRelation,
    Header, IDHeader, IntTier, MainTier, MorTier, MorWord, ParticipantEntry, PhoTier, PhoWord,
    SinTier, SitTier, SpaTier, Utterance, WorTier, Word,
};
use talkbank_model::{ChatParser, FragmentSemanticContext, ParseOutcome};
use talkbank_model::{
    ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation, Span,
};

fn report_context_dependent_main_tier_error(
    errors: &impl ErrorSink,
    input: &str,
    offset: usize,
    span: Span,
) {
    let start = (span.start as usize)
        .saturating_sub(offset)
        .min(input.len());
    let end = (span.end as usize).saturating_sub(offset).min(input.len());
    let snippet = input.get(start..end).unwrap_or(input);

    errors.report(
        ParseError::new(
            ErrorCode::InvalidWordFormat,
            Severity::Error,
            SourceLocation::new(span),
            ErrorContext::new(input, start..end.max(start), snippet),
            "Main-tier fragment requires file context to interpret CA omission shorthand",
        )
        .with_suggestion(
            "Parse a full CHAT file with @Options: CA (or CA-Unicode), or rewrite the fragment using context-independent CHAT notation",
        ),
    );
}

/// Pure chumsky parser implementation for CHAT format.
pub struct DirectParser {}

impl DirectParser {
    /// Create a new direct parser instance.
    pub fn new() -> Result<Self, String> {
        Ok(Self {})
    }
}

impl Default for DirectParser {
    /// Builds a parser with default configuration.
    fn default() -> Self {
        match Self::new() {
            Ok(parser) => parser,
            Err(_) => Self {},
        }
    }
}

// Result API (for compatibility with TreeSitterParser test infrastructure)
impl DirectParser {
    /// Parse a CHAT file with Result API (collects errors into ParseErrors)
    pub fn parse_chat_file(&self, input: &str) -> Result<ChatFile, talkbank_model::ParseErrors> {
        use talkbank_model::{ErrorCode, ErrorCollector, ParseError, Severity, Span};
        let errors = ErrorCollector::new();
        let chat_file = ChatParser::parse_chat_file(self, input, 0, &errors);

        let mut error_vec = errors.into_vec();
        match chat_file {
            ParseOutcome::Parsed(chat_file) => {
                if error_vec.is_empty() {
                    Ok(chat_file)
                } else {
                    Err(talkbank_model::ParseErrors { errors: error_vec })
                }
            }
            ParseOutcome::Rejected => {
                if error_vec.is_empty() {
                    error_vec.push(ParseError::from_source_span(
                        ErrorCode::ParseFailed,
                        Severity::Error,
                        Span::from_usize(0, input.len()),
                        input,
                        input,
                        "Direct parser returned no result and emitted no parse diagnostics",
                    ));
                }
                Err(talkbank_model::ParseErrors { errors: error_vec })
            }
        }
    }
}

impl ChatParser for DirectParser {
    /// Returns the parser identifier used in diagnostics and benchmarks.
    fn parser_name(&self) -> &'static str {
        "direct-parser"
    }

    /// Parses word.
    fn parse_word(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<Word> {
        word::parse_word_impl(input, offset, errors)
    }

    /// Parses mor tier.
    fn parse_mor_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<MorTier> {
        // Use native chumsky implementation for content-only parsing
        mor_tier::parse_mor_tier_content(input, offset, errors)
    }

    /// Parses main tier.
    fn parse_main_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<MainTier> {
        <Self as ChatParser>::parse_main_tier_with_context(
            self,
            input,
            offset,
            &FragmentSemanticContext::default(),
            errors,
        )
    }

    fn parse_main_tier_with_context(
        &self,
        input: &str,
        offset: usize,
        context: &FragmentSemanticContext,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<MainTier> {
        match main_tier::parse_main_tier_impl(input, offset, errors) {
            ParseOutcome::Parsed(mut main) => {
                if context.ca_mode() {
                    file::ca_normalize::normalize_ca_omissions_main_tier(&mut main);
                    ParseOutcome::Parsed(main)
                } else if let Some(span) = main.find_context_dependent_ca_omission_span() {
                    report_context_dependent_main_tier_error(errors, input, offset, span);
                    ParseOutcome::Rejected
                } else {
                    ParseOutcome::Parsed(main)
                }
            }
            ParseOutcome::Rejected => ParseOutcome::Rejected,
        }
    }

    /// Parses utterance.
    fn parse_utterance(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<Utterance> {
        <Self as ChatParser>::parse_utterance_with_context(
            self,
            input,
            offset,
            &FragmentSemanticContext::default(),
            errors,
        )
    }

    fn parse_utterance_with_context(
        &self,
        input: &str,
        offset: usize,
        context: &FragmentSemanticContext,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<Utterance> {
        match main_tier::parse_utterance_impl(input, offset, errors) {
            ParseOutcome::Parsed(mut utterance) => {
                if context.ca_mode() {
                    file::ca_normalize::normalize_ca_omissions_main_tier(&mut utterance.main);
                    ParseOutcome::Parsed(utterance)
                } else if let Some(span) = utterance.main.find_context_dependent_ca_omission_span()
                {
                    report_context_dependent_main_tier_error(errors, input, offset, span);
                    ParseOutcome::Rejected
                } else {
                    ParseOutcome::Parsed(utterance)
                }
            }
            ParseOutcome::Rejected => ParseOutcome::Rejected,
        }
    }

    /// Parses chat file.
    fn parse_chat_file(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ChatFile> {
        // Use our native file parser for TDD-driven development.
        // Features are implemented incrementally as golden tests demand them.
        file::parse_chat_file_impl(input, offset, errors)
    }

    // =========================================================================
    // Headers - Native chumsky implementations
    // =========================================================================

    /// Parses header.
    fn parse_header(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<Header> {
        header::parse_header_impl(input, offset, errors)
    }

    /// Parses id header.
    fn parse_id_header(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<IDHeader> {
        header::parse_id_header_standalone(input, offset, errors)
    }

    /// Parses participant entry.
    fn parse_participant_entry(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ParticipantEntry> {
        header::parse_participant_entry_standalone(input, offset, errors)
    }

    // =========================================================================
    // Morphology - Delegate to TreeSitterParser for now
    // =========================================================================

    /// Parses mor word.
    fn parse_mor_word(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<MorWord> {
        // Use native chumsky implementation
        mor_tier::parse_mor_word_content(input, offset, errors)
    }

    // =========================================================================
    // Grammar - Delegate to TreeSitterParser for now
    // =========================================================================

    /// Parses gra tier.
    fn parse_gra_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<GraTier> {
        // Use native chumsky implementation for content-only parsing
        gra_tier::parse_gra_tier_content(input, offset, errors)
    }

    /// Parses gra relation.
    fn parse_gra_relation(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<GrammaticalRelation> {
        // Use native chumsky implementation
        gra_tier::parse_gra_relation_content(input, offset, errors)
    }

    // =========================================================================
    // Phonology - Delegate to TreeSitterParser for now
    // =========================================================================

    /// Parses pho tier.
    fn parse_pho_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<PhoTier> {
        // Use native chumsky implementation for content-only parsing
        pho_tier::parse_pho_tier_content(input, offset, errors)
    }

    /// Parses pho word.
    fn parse_pho_word(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<PhoWord> {
        // Use native chumsky implementation
        pho_tier::parse_pho_word_content(input, offset, errors)
    }

    // =========================================================================
    // Gesture/Action - Delegate to TreeSitterParser for now
    // =========================================================================

    /// Parses sin tier.
    fn parse_sin_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<SinTier> {
        // Use native chumsky implementation
        sin_tier::parse_sin_tier_content(input, offset, errors)
    }

    /// Parses act tier.
    fn parse_act_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ActTier> {
        // API contract: expects content-only input (without %act:\t prefix)
        text_tier::parse_act_tier_content(input, offset, errors)
    }

    /// Parses cod tier.
    fn parse_cod_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<CodTier> {
        // API contract: expects content-only input (without %cod:\t prefix)
        text_tier::parse_cod_tier_content(input, offset, errors)
    }

    // =========================================================================
    // Text Tiers - Delegate to TreeSitterParser for now
    // =========================================================================

    /// Parses com tier.
    fn parse_com_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ComTier> {
        // IMPORTANT: ChatParser API CONTRACT - expects content-only input
        // The input should be the comment tier content WITHOUT the %com:\t prefix.
        // Example: "after transcribing, I realized\n\tthis means something"
        // NOT: "%com:\tafter transcribing, I realized\n\tthis means something"
        //
        // Why: TreeSitterParser's wrapper_parse_tier() internally adds the prefix.
        // If input already had the prefix, wrapper_parse_tier would create invalid CHAT.
        // This convention ensures consistency between DirectParser and TreeSitterParser.
        //
        // See: API_PREFIX_CONVENTIONS.md for complete documentation
        text_tier::parse_com_tier_content(input, offset, errors)
    }

    /// Parses exp tier.
    fn parse_exp_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ExpTier> {
        // NOTE: Content-only input expected (without %exp:\t prefix)
        // See parse_com_tier() for rationale - applies to all text tiers
        text_tier::parse_exp_tier_content(input, offset, errors)
    }

    /// Parses add tier.
    fn parse_add_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<AddTier> {
        // Strip %add:\t prefix if present, then parse content
        text_tier::parse_add_tier_content(input, offset, errors)
    }

    /// Parses gpx tier.
    fn parse_gpx_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<GpxTier> {
        // Strip %gpx:\t prefix if present, then parse content
        text_tier::parse_gpx_tier_content(input, offset, errors)
    }

    /// Parses int tier.
    fn parse_int_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<IntTier> {
        // API contract: expects content-only input (without %int:\t prefix)
        text_tier::parse_int_tier_content(input, offset, errors)
    }

    /// Parses spa tier.
    fn parse_spa_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<SpaTier> {
        // API contract: expects content-only input (without %spa:\t prefix)
        text_tier::parse_spa_tier_content(input, offset, errors)
    }

    /// Parses sit tier.
    fn parse_sit_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<SitTier> {
        // API contract: expects content-only input (without %sit:\t prefix)
        text_tier::parse_sit_tier_content(input, offset, errors)
    }

    /// Parses wor tier.
    fn parse_wor_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<WorTier> {
        // Use native chumsky implementation
        wor_tier::parse_wor_tier_content(input, offset, errors)
    }

    // =========================================================================
    // Generic Dependent Tier - Delegate to TreeSitterParser for now
    // =========================================================================

    /// Parses dependent tier.
    fn parse_dependent_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<DependentTier> {
        // Use native chumsky implementation
        dependent_tier::parse_dependent_tier_impl(input, offset, errors)
    }
}
