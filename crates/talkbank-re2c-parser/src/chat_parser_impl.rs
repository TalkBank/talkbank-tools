//! Implementation of the `ChatParser` trait for the re2c-based parser.
//!
//! This makes our parser a drop-in replacement for `TreeSitterParser`
//! via the shared `ChatParser` trait. Shared tests verify both parsers
//! produce semantically equivalent output.

use talkbank_model::model::{
    ActTier, AddTier, ChatFile as ModelChatFile, CodTier, ComTier,
    DependentTier as ModelDependentTier, ExpTier, GpxTier, GraTier as ModelGraTier,
    GrammaticalRelation, Header, IDHeader, IntTier, MainTier as ModelMainTier,
    MorTier as ModelMorTier, MorWord, ParticipantEntry, PhoTier as ModelPhoTier, PhoWord, SitTier,
    SpaTier, Utterance as ModelUtterance, WorTier, Word,
};
use talkbank_model::{ChatParser, ErrorSink, FragmentSemanticContext, ParseOutcome, Span};

/// Re2c-based CHAT parser implementing the shared `ChatParser` trait.
///
/// Unlike `TreeSitterParser`, this parser uses re2c for lexing and
/// a handwritten recursive-descent parser. It does not require
/// tree-sitter to be installed or configured.
pub struct Re2cParser;

impl Re2cParser {
    /// Create a new re2c parser instance.
    ///
    /// Unlike `TreeSitterParser`, this is zero-cost — no grammar loading,
    /// no internal buffers. The parser is stateless and `Send + Sync`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for Re2cParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatParser for Re2cParser {
    fn parser_name(&self) -> &'static str {
        "Re2cParser"
    }

    fn parse_chat_file(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<ModelChatFile> {
        let parsed = crate::parser::parse_chat_file(input);
        ParseOutcome::parsed(ModelChatFile::from(&parsed))
    }

    fn parse_header(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<Header> {
        // Parse as a single-line file and extract the first header
        let parsed = crate::parser::parse_chat_file(input);
        for line in &parsed.lines {
            if let crate::ast::Line::Header(h) = line {
                return ParseOutcome::parsed(crate::convert::header_parsed_to_model(h));
            }
        }
        ParseOutcome::rejected()
    }

    fn parse_id_header(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<IDHeader> {
        match crate::parser::parse_id_header(input) {
            Some(parsed) => ParseOutcome::parsed(IDHeader::from(&parsed)),
            None => ParseOutcome::rejected(),
        }
    }

    fn parse_participant_entry(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<ParticipantEntry> {
        let parsed = crate::parser::parse_participants_header(input);
        match parsed.entries.first() {
            Some(entry) => ParseOutcome::parsed(ParticipantEntry::from(entry)),
            None => ParseOutcome::rejected(),
        }
    }

    fn parse_utterance(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<ModelUtterance> {
        // Parse as a file fragment and extract the first utterance
        let parsed = crate::parser::parse_chat_file(input);
        for line in &parsed.lines {
            if let crate::ast::Line::Utterance(u) = line {
                return ParseOutcome::parsed(ModelUtterance::from(u));
            }
        }
        ParseOutcome::rejected()
    }

    fn parse_main_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<ModelMainTier> {
        match crate::parser::parse_main_tier(input) {
            Some(parsed) => ParseOutcome::parsed(ModelMainTier::from(&parsed)),
            None => ParseOutcome::rejected(),
        }
    }

    fn parse_word(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<Word> {
        match crate::parser::parse_word(input) {
            Some(parsed) => ParseOutcome::parsed(Word::from(&parsed)),
            None => ParseOutcome::rejected(),
        }
    }

    fn parse_mor_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<ModelMorTier> {
        let parsed = crate::parser::parse_mor_tier(input);
        ParseOutcome::parsed(ModelMorTier::from(&parsed))
    }

    fn parse_mor_word(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<MorWord> {
        match crate::parser::parse_mor_word(input) {
            Some(parsed) => ParseOutcome::parsed(MorWord::from(&parsed)),
            None => ParseOutcome::rejected(),
        }
    }

    fn parse_gra_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<ModelGraTier> {
        let parsed = crate::parser::parse_gra_tier(input);
        ParseOutcome::parsed(ModelGraTier::from(&parsed))
    }

    fn parse_gra_relation(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<GrammaticalRelation> {
        match crate::parser::parse_gra_relation(input) {
            Some(parsed) => ParseOutcome::parsed(GrammaticalRelation::from(&parsed)),
            None => ParseOutcome::rejected(),
        }
    }

    fn parse_pho_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<ModelPhoTier> {
        let parsed = crate::parser::parse_pho_tier(input);
        ParseOutcome::parsed(ModelPhoTier::from(&parsed))
    }

    fn parse_pho_word(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<PhoWord> {
        // Parse as a single-word pho tier, extract first word
        let parsed = crate::parser::parse_pho_tier(input);
        let first_word = parsed.items.iter().find_map(|item| match item {
            crate::ast::PhoItemParsed::Word(w) => Some(w),
            _ => None,
        });
        match first_word {
            Some(w) => ParseOutcome::parsed(PhoWord::from(w)),
            None => ParseOutcome::rejected(),
        }
    }

    fn parse_sin_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<talkbank_model::model::SinTier> {
        ParseOutcome::parsed(crate::convert::sin_tier_from_text(input))
    }

    fn parse_act_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<ActTier> {
        let parsed = crate::parser::parse_text_tier(input);
        ParseOutcome::parsed(crate::convert::to_act_tier(&parsed))
    }

    fn parse_cod_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<CodTier> {
        let parsed = crate::parser::parse_text_tier(input);
        ParseOutcome::parsed(crate::convert::to_cod_tier(&parsed))
    }

    fn parse_com_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<ComTier> {
        let parsed = crate::parser::parse_text_tier(input);
        ParseOutcome::parsed(crate::convert::to_com_tier(&parsed))
    }

    fn parse_exp_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<ExpTier> {
        let parsed = crate::parser::parse_text_tier(input);
        ParseOutcome::parsed(crate::convert::to_exp_tier(&parsed))
    }

    fn parse_add_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<AddTier> {
        let parsed = crate::parser::parse_text_tier(input);
        ParseOutcome::parsed(crate::convert::to_add_tier(&parsed))
    }

    fn parse_gpx_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<GpxTier> {
        let parsed = crate::parser::parse_text_tier(input);
        ParseOutcome::parsed(crate::convert::to_gpx_tier(&parsed))
    }

    fn parse_int_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<IntTier> {
        let parsed = crate::parser::parse_text_tier(input);
        ParseOutcome::parsed(crate::convert::to_int_tier(&parsed))
    }

    fn parse_spa_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<SpaTier> {
        let parsed = crate::parser::parse_text_tier(input);
        ParseOutcome::parsed(crate::convert::to_spa_tier(&parsed))
    }

    fn parse_sit_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<SitTier> {
        let parsed = crate::parser::parse_text_tier(input);
        ParseOutcome::parsed(crate::convert::to_sit_tier(&parsed))
    }

    fn parse_wor_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<WorTier> {
        ParseOutcome::parsed(crate::convert::wor_tier_from_input(input))
    }

    fn parse_dependent_tier(
        &self,
        input: &str,
        _offset: usize,
        _errors: &impl ErrorSink,
    ) -> ParseOutcome<ModelDependentTier> {
        // Parse as a file fragment and extract the first dependent tier from the first utterance
        let parsed = crate::parser::parse_chat_file(input);
        for line in &parsed.lines {
            if let crate::ast::Line::Utterance(u) = line {
                if let Some(tier) = u.dependent_tiers.first() {
                    return ParseOutcome::parsed(crate::convert::dependent_tier_to_model(tier));
                }
            }
        }
        ParseOutcome::rejected()
    }
}
