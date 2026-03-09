//! File-level CHAT parser that builds [`ChatFile`] from raw text.
//!
//! This parser reads headers, main tiers, and dependent tiers in source order,
//! then groups each main tier with following dependent tiers into utterances.
//! Continuation markers (`\n\t`, `\r\n\t`) are preserved and interpreted by
//! tier-specific whitespace parsers.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#CA_Option>

pub(crate) mod ca_normalize;
mod grouping;
mod participants;
mod tier_parser;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use chumsky::prelude::*;
use talkbank_model::ParseOutcome;
use talkbank_model::model::{ChatFile, Header};
use talkbank_model::{ErrorCode, ErrorSink, LineMap, ParseError, Severity, Span};

use ca_normalize::{headers_enable_ca_mode, normalize_ca_omissions};
use grouping::{group_tiers_into_file, parse_all_tier_contents};
use participants::build_participants;
use tier_parser::file_tiers_parser;

/// Parse a CHAT format file using pure chumsky combinators.
///
/// Strategy:
/// 1. Parse file structure with chumsky (tiers with embedded \n\t continuations)
/// 2. Parse each tier's content with tier-specific parsers (using ws_parser())
/// 3. Group main tiers with following dependent tiers into Utterances
/// 4. Build ChatFile with all lines
pub fn parse_chat_file_impl(
    input: &str,
    _offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<ChatFile> {
    // Phase 1: Parse file structure with chumsky
    let raw_tiers = match file_tiers_parser().parse(input).into_result() {
        Ok(tiers) => tiers,
        Err(parse_errors) => {
            // Report chumsky parse errors
            for err in parse_errors {
                errors.report(ParseError::from_source_span(
                    ErrorCode::InvalidLineFormat,
                    Severity::Error,
                    Span::from_usize(err.span().start, err.span().end),
                    input,
                    input,
                    format!("Parse error: {:?}", err.reason()),
                ));
            }
            return ParseOutcome::rejected();
        }
    };

    // Phase 2: Parse each tier's content
    let Some(parsed_tiers) = parse_all_tier_contents(&raw_tiers, errors) else {
        return ParseOutcome::rejected();
    };

    // Phase 3: Group into utterances
    let mut chat_file_lines = group_tiers_into_file(parsed_tiers, errors);

    // Phase 4: Build participants and CA normalization
    let headers: Vec<Header> = chat_file_lines
        .iter()
        .filter_map(|line| line.as_header().cloned())
        .collect();

    let (participants, participant_errors) = build_participants(&headers);
    for err in participant_errors {
        errors.report(err);
    }

    if headers_enable_ca_mode(&headers) {
        normalize_ca_omissions(&mut chat_file_lines);
    }

    // Phase 5: Build ChatFile
    ParseOutcome::parsed(ChatFile::with_line_map(
        chat_file_lines,
        participants,
        LineMap::new(input),
    ))
}
