use chumsky::prelude::*;
use talkbank_model::model::{Header, IDHeader, ParticipantEntry};
use talkbank_model::{ErrorCode, ErrorSink, ParseError, ParseOutcome, Severity, Span};

use super::complex::{id_header_parser, participant_entry_parser};
use super::helpers::report_parse_errors;

/// Parse standalone @ID header content (API compatibility).
pub fn parse_id_header_standalone(
    input: &str,
    _offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<IDHeader> {
    let parser = id_header_parser();
    match parser.parse(input).into_result() {
        Ok(Header::ID(id)) => ParseOutcome::parsed(id),
        Ok(_) => {
            report_expected_id_header(input, errors);
            ParseOutcome::rejected()
        }
        Err(parse_errors) => {
            report_parse_errors(parse_errors, input, 0, "ID header parse error", errors);
            ParseOutcome::rejected()
        }
    }
}

/// Parse standalone participant entry (API compatibility).
pub fn parse_participant_entry_standalone(
    input: &str,
    _offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<ParticipantEntry> {
    let parser = participant_entry_parser();
    match parser.parse(input).into_result() {
        Ok(entry) => ParseOutcome::parsed(entry),
        Err(parse_errors) => {
            report_parse_errors(
                parse_errors,
                input,
                0,
                "Participant entry parse error",
                errors,
            );
            ParseOutcome::rejected()
        }
    }
}

fn report_expected_id_header(input: &str, errors: &impl ErrorSink) {
    errors.report(ParseError::from_source_span(
        ErrorCode::new("E501"),
        Severity::Error,
        Span::from_usize(0, input.len()),
        input,
        input,
        "Expected @ID header".to_string(),
    ));
}
