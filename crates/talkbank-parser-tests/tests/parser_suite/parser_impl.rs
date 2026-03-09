//! Parser implementation wrapper and suite constructor.
//!
//! Provides `ParserImpl` — a delegating `ChatParser` implementation that
//! wraps both `TreeSitterParser` and `DirectParser` for cross-backend
//! integration testing.

use talkbank_direct_parser::DirectParser;
use talkbank_model::ErrorSink;
use talkbank_model::dependent_tier::DependentTier;
use talkbank_model::model::{
    ActTier, AddTier, ChatFile, CodTier, ComTier, ExpTier, GpxTier, GraTier, GrammaticalRelation,
    Header, IDHeader, IntTier, MainTier, MorTier, MorWord, ParticipantEntry, PhoTier, PhoWord,
    SinTier, SitTier, SpaTier, Utterance, WorTier, Word,
};
use talkbank_model::{ChatParser, FragmentSemanticContext, ParseOutcome};
use talkbank_parser::TreeSitterParser;
use talkbank_parser_tests::test_error::TestError;

pub const SAMPLE_WORD_COUNT: usize = 3;
pub const MOR_TIER_INPUT: &str = "pro|I v|want n|cookie-PL .";
pub const UTTERANCE_INPUT: &str = "*CHI:\tI want .\n%mor:\tpro|I v|want n|cookie-PL";

/// Parser backend used by cross-implementation integration tests.
pub enum ParserImpl {
    TreeSitter(TreeSitterParser),
    Direct(DirectParser),
}

impl ChatParser for ParserImpl {
    /// Returns the parser backend name.
    fn parser_name(&self) -> &'static str {
        match self {
            ParserImpl::TreeSitter(tree) => tree.parser_name(),
            ParserImpl::Direct(direct) => direct.parser_name(),
        }
    }

    /// Parses word.
    fn parse_word(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<Word> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_word(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_word(direct, input, 0, errors),
        }
    }

    /// Parses mor tier.
    fn parse_mor_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<MorTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_mor_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_mor_tier(direct, input, 0, errors),
        }
    }

    /// Parses main tier.
    fn parse_main_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<MainTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_main_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_main_tier(direct, input, 0, errors),
        }
    }

    fn parse_main_tier_with_context(
        &self,
        input: &str,
        offset: usize,
        context: &FragmentSemanticContext,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<MainTier> {
        match self {
            ParserImpl::TreeSitter(tree) => {
                ChatParser::parse_main_tier_with_context(tree, input, 0, context, errors)
            }
            ParserImpl::Direct(direct) => {
                ChatParser::parse_main_tier_with_context(direct, input, 0, context, errors)
            }
        }
    }

    /// Parses utterance.
    fn parse_utterance(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<Utterance> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_utterance(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_utterance(direct, input, 0, errors),
        }
    }

    fn parse_utterance_with_context(
        &self,
        input: &str,
        offset: usize,
        context: &FragmentSemanticContext,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<Utterance> {
        match self {
            ParserImpl::TreeSitter(tree) => {
                ChatParser::parse_utterance_with_context(tree, input, 0, context, errors)
            }
            ParserImpl::Direct(direct) => {
                ChatParser::parse_utterance_with_context(direct, input, 0, context, errors)
            }
        }
    }

    /// Parses chat file.
    fn parse_chat_file(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ChatFile> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_chat_file(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_chat_file(direct, input, 0, errors),
        }
    }

    // Headers
    /// Parses header.
    fn parse_header(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<Header> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_header(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_header(direct, input, 0, errors),
        }
    }

    /// Parses id header.
    fn parse_id_header(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<IDHeader> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_id_header(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_id_header(direct, input, 0, errors),
        }
    }

    /// Parses participant entry.
    fn parse_participant_entry(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ParticipantEntry> {
        match self {
            ParserImpl::TreeSitter(tree) => {
                ChatParser::parse_participant_entry(tree, input, 0, errors)
            }
            ParserImpl::Direct(direct) => {
                ChatParser::parse_participant_entry(direct, input, 0, errors)
            }
        }
    }

    // Morphology
    /// Parses mor word.
    fn parse_mor_word(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<MorWord> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_mor_word(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_mor_word(direct, input, 0, errors),
        }
    }

    // Grammar
    /// Parses gra tier.
    fn parse_gra_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<GraTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_gra_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_gra_tier(direct, input, 0, errors),
        }
    }

    /// Parses gra relation.
    fn parse_gra_relation(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<GrammaticalRelation> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_gra_relation(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_gra_relation(direct, input, 0, errors),
        }
    }

    // Phonology
    /// Parses pho tier.
    fn parse_pho_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<PhoTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_pho_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_pho_tier(direct, input, 0, errors),
        }
    }

    /// Parses pho word.
    fn parse_pho_word(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<PhoWord> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_pho_word(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_pho_word(direct, input, 0, errors),
        }
    }

    // Gesture/Action
    /// Parses sin tier.
    fn parse_sin_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<SinTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_sin_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_sin_tier(direct, input, 0, errors),
        }
    }

    /// Parses act tier.
    fn parse_act_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ActTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_act_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_act_tier(direct, input, 0, errors),
        }
    }

    /// Parses cod tier.
    fn parse_cod_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<CodTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_cod_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_cod_tier(direct, input, 0, errors),
        }
    }

    // Text Tiers
    /// Parses com tier.
    fn parse_com_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ComTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_com_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_com_tier(direct, input, 0, errors),
        }
    }

    /// Parses exp tier.
    fn parse_exp_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ExpTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_exp_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_exp_tier(direct, input, 0, errors),
        }
    }

    /// Parses add tier.
    fn parse_add_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<AddTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_add_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_add_tier(direct, input, 0, errors),
        }
    }

    /// Parses gpx tier.
    fn parse_gpx_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<GpxTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_gpx_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_gpx_tier(direct, input, 0, errors),
        }
    }

    /// Parses int tier.
    fn parse_int_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<IntTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_int_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_int_tier(direct, input, 0, errors),
        }
    }

    /// Parses spa tier.
    fn parse_spa_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<SpaTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_spa_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_spa_tier(direct, input, 0, errors),
        }
    }

    /// Parses sit tier.
    fn parse_sit_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<SitTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_sit_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_sit_tier(direct, input, 0, errors),
        }
    }

    /// Parses wor tier.
    fn parse_wor_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<WorTier> {
        match self {
            ParserImpl::TreeSitter(tree) => ChatParser::parse_wor_tier(tree, input, 0, errors),
            ParserImpl::Direct(direct) => ChatParser::parse_wor_tier(direct, input, 0, errors),
        }
    }

    // Generic
    /// Parses dependent tier.
    fn parse_dependent_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<DependentTier> {
        match self {
            ParserImpl::TreeSitter(tree) => {
                ChatParser::parse_dependent_tier(tree, input, 0, errors)
            }
            ParserImpl::Direct(direct) => {
                ChatParser::parse_dependent_tier(direct, input, 0, errors)
            }
        }
    }
}

/// Builds the parser suite used in cross-backend integration tests.
pub fn parser_suite() -> Result<Vec<ParserImpl>, TestError> {
    let tree_sitter =
        TreeSitterParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let direct = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    Ok(vec![
        ParserImpl::TreeSitter(tree_sitter),
        ParserImpl::Direct(direct),
    ])
}
