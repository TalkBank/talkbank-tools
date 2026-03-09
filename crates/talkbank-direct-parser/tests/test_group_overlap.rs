//! Test module for test group overlap in `talkbank-chat`.
//!
//! These tests document expected behavior and regressions.

// Test parsing groups with overlap markers

use talkbank_direct_parser::DirectParser;
use talkbank_parser_tests::test_error::TestError;

/// Tests group with overlap markers.
#[test]
fn test_group_with_overlap_markers() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|test|CHI|||||Child|||\n*CHI:\t<a ⌈ top begin overlap , top end overlap ⌉ here> [= foo] .\n@End\n";

    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let result = parser.parse_chat_file(input);

    println!("Parse result: {:?}", result);

    let file = result.map_err(|err| {
        println!("Parse errors: {:?}", err);
        TestError::Failure("Should not have parse errors".to_string())
    })?;
    let utterances: Vec<_> = file.utterances().collect();
    println!("Utterances: {}", utterances.len());
    if utterances.len() != 1 {
        return Err(TestError::Failure("Should parse 1 utterance".to_string()));
    }

    Ok(())
}
