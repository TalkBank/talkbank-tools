//! ChatParser trait implementation for TreeSitterParser
//!
//! This module implements the `ChatParser` trait, providing a unified API
//! for parsing all CHAT constructs using tree-sitter.
//!
//! CHAT reference anchors:
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//!
//! # Architecture
//!
//! Most tier parsing follows the "minimal wrapper" pattern:
//! 1. Wrap tier content in a minimal valid CHAT document
//! 2. Parse the wrapper with tree-sitter
//! 3. Extract the tier from the parsed file
//! 4. Adjust spans from wrapper-relative to document-absolute
//!
//! This pattern is encapsulated in `parser_impl::wrapper_parse_tier()`.

use talkbank_model::dependent_tier::DependentTier;
use talkbank_model::model::{
    ActTier, AddTier, ChatFile, CodTier, ComTier, ExpTier, GpxTier, GraTier, GrammaticalRelation,
    Header, IDHeader, IntTier, MainTier, MorTier, MorWord, ParticipantEntry, PhoTier, PhoWord,
    SinTier, SitTier, SpaTier, Utterance, WorTier, Word,
};
use talkbank_model::{ChatParser, FragmentSemanticContext, ParseOutcome};
use talkbank_model::{
    ErrorCode, ErrorContext, ErrorSink, OffsetAdjustingErrorSink, ParseError, ParseErrors,
    Severity, SourceLocation, Span, SpanShift,
};

use super::parser_impl::{wrapper_parse_generic_tier, wrapper_parse_tier};
use crate::parser::TreeSitterParser;
use crate::parser::chat_file_parser::MINIMAL_CHAT_PREFIX;

/// Report parse errors through an ErrorSink.
fn report_parse_errors(errors: ParseErrors, sink: &impl ErrorSink) {
    sink.report_vec(errors.into_error_vec());
}

const PARTICIPANTS_HEADER_PREFIX: &str = "@Participants:\t";

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

impl ChatParser for TreeSitterParser {
    fn parser_name(&self) -> &'static str {
        "tree-sitter-parser"
    }

    // =========================================================================
    // Word-Level Parsing
    // =========================================================================

    fn parse_word(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<Word> {
        let adjusting_sink = OffsetAdjustingErrorSink::new(errors, offset, input);
        match self.parse_word(input) {
            Ok(mut word) => {
                // `TreeSitterParser::parse_word` parses inside a synthetic file context.
                // Shift spans back to caller-relative coordinates, then apply `offset`
                // so downstream diagnostics map into the original document.
                let wrapper_prefix_len = MINIMAL_CHAT_PREFIX.len() + "*CHI:\t".len();
                word.shift_spans_after(0, -(wrapper_prefix_len as i32) + offset as i32);
                ParseOutcome::parsed(word)
            }
            Err(errs) => {
                report_parse_errors(errs, &adjusting_sink);
                ParseOutcome::rejected()
            }
        }
    }

    // =========================================================================
    // Main Tier Parsing
    // =========================================================================

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
        let adjusting_sink = OffsetAdjustingErrorSink::new(errors, offset, input);
        match self.parse_main_tier(input) {
            Ok(mut main) => {
                // Main-tier parsing also uses a synthetic file prefix internally.
                let wrapper_prefix_len = MINIMAL_CHAT_PREFIX.len();
                main.shift_spans_after(0, -(wrapper_prefix_len as i32) + offset as i32);
                if context.ca_mode() {
                    crate::parser::chat_file_parser::chat_file::normalize::normalize_ca_omissions_main_tier(&mut main);
                    ParseOutcome::parsed(main)
                } else if let Some(span) = main.find_context_dependent_ca_omission_span() {
                    report_context_dependent_main_tier_error(errors, input, offset, span);
                    ParseOutcome::rejected()
                } else {
                    ParseOutcome::parsed(main)
                }
            }
            Err(errs) => {
                report_parse_errors(errs, &adjusting_sink);
                ParseOutcome::rejected()
            }
        }
    }

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
        let adjusting_sink = OffsetAdjustingErrorSink::new(errors, offset, input);
        match self.parse_utterance(input) {
            Ok(mut utterance) => {
                utterance.shift_spans_after(0, offset as i32);
                if context.ca_mode() {
                    crate::parser::chat_file_parser::chat_file::normalize::normalize_ca_omissions_main_tier(&mut utterance.main);
                    ParseOutcome::parsed(utterance)
                } else if let Some(span) = utterance.main.find_context_dependent_ca_omission_span()
                {
                    report_context_dependent_main_tier_error(errors, input, offset, span);
                    ParseOutcome::rejected()
                } else {
                    ParseOutcome::parsed(utterance)
                }
            }
            Err(errs) => {
                report_parse_errors(errs, &adjusting_sink);
                ParseOutcome::rejected()
            }
        }
    }

    fn parse_chat_file(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ChatFile> {
        let adjusting_sink = OffsetAdjustingErrorSink::new(errors, offset, input);
        // Streaming parse always returns a recovered ChatFile and reports issues
        // through `adjusting_sink`, so the outcome stays `Parsed`.
        let chat = self.parse_chat_file_streaming(input, &adjusting_sink);
        ParseOutcome::parsed(chat)
    }

    // =========================================================================
    // Headers
    // =========================================================================

    fn parse_header(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<Header> {
        let adjusting_sink = OffsetAdjustingErrorSink::new(errors, offset, input);
        match self.parse_header(input) {
            Ok(mut header) => {
                header.shift_spans_after(0, offset as i32);
                ParseOutcome::parsed(header)
            }
            Err(parse_errors) => {
                for error in parse_errors.errors {
                    adjusting_sink.report(error);
                }
                ParseOutcome::rejected()
            }
        }
    }

    fn parse_id_header(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<IDHeader> {
        match ChatParser::parse_header(self, input, offset, errors) {
            ParseOutcome::Parsed(Header::ID(id)) => ParseOutcome::parsed(id),
            _ => ParseOutcome::rejected(),
        }
    }

    fn parse_participant_entry(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ParticipantEntry> {
        // Parse a bare participant entry by embedding it in a valid header line.
        let wrapper = format!("{PARTICIPANTS_HEADER_PREFIX}{}\n", input);
        let Some(header) = ChatParser::parse_header(self, &wrapper, 0, errors).into_option() else {
            return ParseOutcome::rejected();
        };
        match header {
            Header::Participants { entries } => {
                let Some(mut entry) = entries.into_iter().next() else {
                    return ParseOutcome::rejected();
                };
                // Remove synthetic header prefix, then apply caller offset.
                entry.shift_spans_after(
                    0,
                    -(PARTICIPANTS_HEADER_PREFIX.len() as i32) + offset as i32,
                );
                ParseOutcome::parsed(entry)
            }
            _ => ParseOutcome::rejected(),
        }
    }

    // =========================================================================
    // Morphology Tiers
    // =========================================================================

    fn parse_mor_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<MorTier> {
        wrapper_parse_tier(self, "%mor:\t", input, offset, errors, |tier| match tier {
            DependentTier::Mor(tier) => Some(tier),
            _ => None,
        })
    }

    fn parse_mor_word(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<MorWord> {
        // `%mor` parser expects tier-level structure, so single items are parsed
        // via a synthetic tier line and then projected back to one MorWord.
        let Some(tier) =
            ChatParser::parse_mor_tier(self, &format!("{} .", input), offset, errors).into_option()
        else {
            return ParseOutcome::rejected();
        };

        // MorTier.items contains Mor objects with MorWord directly in main field
        let Some(mor) = tier.items.0.into_iter().next() else {
            return ParseOutcome::rejected();
        };
        ParseOutcome::parsed(mor.main)
    }

    // =========================================================================
    // Grammar Tiers
    // =========================================================================

    fn parse_gra_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<GraTier> {
        wrapper_parse_tier(self, "%gra:\t", input, offset, errors, |tier| match tier {
            DependentTier::Gra(tier) => Some(tier),
            _ => None,
        })
    }

    fn parse_gra_relation(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<GrammaticalRelation> {
        // GRA tier validation expects at least one well-formed relation sequence.
        // Add a minimal sentinel relation so one standalone relation can be parsed.
        let Some(tier) =
            ChatParser::parse_gra_tier(self, &format!("{} 0|0|PUNCT", input), offset, errors)
                .into_option()
        else {
            return ParseOutcome::rejected();
        };
        tier.relations.0.into_iter().next().into()
    }

    // =========================================================================
    // Phonology Tiers
    // =========================================================================

    fn parse_pho_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<PhoTier> {
        wrapper_parse_tier(self, "%pho:\t", input, offset, errors, |tier| match tier {
            DependentTier::Pho(tier) => Some(tier),
            _ => None,
        })
    }

    fn parse_pho_word(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<PhoWord> {
        use talkbank_model::model::PhoItem;

        // Parse a single PHO token by embedding it into a valid `%pho` tier.
        let Some(tier) =
            ChatParser::parse_pho_tier(self, &format!("{} .", input), offset, errors).into_option()
        else {
            return ParseOutcome::rejected();
        };

        // PhoTier.items is PhoItems(Vec<PhoItem>), access with .0
        let Some(item) = tier.items.0.into_iter().next() else {
            return ParseOutcome::rejected();
        };
        match item {
            PhoItem::Word(word) => ParseOutcome::parsed(word),
            PhoItem::Group(_) => ParseOutcome::rejected(), // Not a single PhoWord
        }
    }

    // =========================================================================
    // Gesture/Action Tiers
    // =========================================================================

    fn parse_sin_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<SinTier> {
        wrapper_parse_tier(self, "%sin:\t", input, offset, errors, |tier| match tier {
            DependentTier::Sin(tier) => Some(tier),
            _ => None,
        })
    }

    fn parse_act_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ActTier> {
        wrapper_parse_tier(self, "%act:\t", input, offset, errors, |tier| match tier {
            DependentTier::Act(act) => Some(act),
            _ => None,
        })
    }

    fn parse_cod_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<CodTier> {
        wrapper_parse_tier(self, "%cod:\t", input, offset, errors, |tier| match tier {
            DependentTier::Cod(cod) => Some(cod),
            _ => None,
        })
    }

    // =========================================================================
    // Text/Commentary Tiers
    // =========================================================================

    fn parse_com_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ComTier> {
        wrapper_parse_tier(self, "%com:\t", input, offset, errors, |tier| match tier {
            DependentTier::Com(com) => Some(com),
            _ => None,
        })
    }

    fn parse_exp_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<ExpTier> {
        wrapper_parse_tier(self, "%exp:\t", input, offset, errors, |tier| match tier {
            DependentTier::Exp(exp) => Some(exp),
            _ => None,
        })
    }

    fn parse_add_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<AddTier> {
        wrapper_parse_tier(self, "%add:\t", input, offset, errors, |tier| match tier {
            DependentTier::Add(add) => Some(add),
            _ => None,
        })
    }

    fn parse_gpx_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<GpxTier> {
        wrapper_parse_tier(self, "%gpx:\t", input, offset, errors, |tier| match tier {
            DependentTier::Gpx(gpx) => Some(gpx),
            _ => None,
        })
    }

    fn parse_int_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<IntTier> {
        wrapper_parse_tier(self, "%int:\t", input, offset, errors, |tier| match tier {
            DependentTier::Int(int) => Some(int),
            _ => None,
        })
    }

    fn parse_spa_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<SpaTier> {
        wrapper_parse_tier(self, "%spa:\t", input, offset, errors, |tier| match tier {
            DependentTier::Spa(spa) => Some(spa),
            _ => None,
        })
    }

    fn parse_sit_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<SitTier> {
        wrapper_parse_tier(self, "%sit:\t", input, offset, errors, |tier| match tier {
            DependentTier::Sit(sit) => Some(sit),
            _ => None,
        })
    }

    fn parse_wor_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<WorTier> {
        wrapper_parse_tier(self, "%wor:\t", input, offset, errors, |tier| match tier {
            DependentTier::Wor(tier) => Some(tier),
            _ => None,
        })
    }

    // =========================================================================
    // Generic Dependent Tier
    // =========================================================================

    fn parse_dependent_tier(
        &self,
        input: &str,
        offset: usize,
        errors: &impl ErrorSink,
    ) -> ParseOutcome<DependentTier> {
        // Unlike typed tier entry points, this input already includes `%label:\t`.
        // Keep the label and content as-is and parse through the generic dispatcher.
        wrapper_parse_generic_tier(self, input, offset, errors)
    }
}
