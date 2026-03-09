//! Test module for test main tier terminator in `talkbank-chat`.
//!
//! These tests document expected behavior and regressions.

// Test to isolate the main tier parsing issue with terminators

use chumsky::prelude::*;
use talkbank_direct_parser::DirectParser;
use talkbank_parser_tests::test_error::TestError;

/// Tests file with whitespace before terminator.
#[test]
fn test_file_with_whitespace_before_terminator() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|test|CHI|||||Child|||\n*CHI:\tword .\n@End\n";
    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let result = parser.parse_chat_file(input);

    println!("Input: {:?}", input);
    println!("Result: {:?}", result);

    let file = result.map_err(|_| {
        TestError::Failure("Should parse file with whitespace before terminator".to_string())
    })?;
    if file.utterances().count() != 1 {
        return Err(TestError::Failure("Should have one utterance".to_string()));
    }
    Ok(())
}

/// Tests file without whitespace before terminator.
#[test]
fn test_file_without_whitespace_before_terminator() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|test|CHI|||||Child|||\n*CHI:\tword?\n@End\n";
    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let result = parser.parse_chat_file(input);

    println!("Input: {:?}", input);
    match &result {
        Ok(file) => {
            println!("File parsed successfully!");
            let utts: Vec<_> = file.utterances().collect();
            println!("Utterances count: {}", utts.len());
            if utts.is_empty() {
                println!("ERROR: No utterances parsed!");
            } else {
                println!("Utterance: {:?}", utts[0]);
            }
        }
        Err(e) => {
            println!("Parse errors: {:?}", e);
        }
    }

    let file = result.map_err(|_| {
        TestError::Failure("Should parse file without whitespace before terminator".to_string())
    })?;
    if file.utterances().count() != 1 {
        return Err(TestError::Failure("Should have one utterance".to_string()));
    }
    Ok(())
}

/// Tests file with language marker and terminators.
#[test]
fn test_file_with_language_marker_and_terminators() -> Result<(), TestError> {
    let input1 = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|test|CHI|||||Child|||\n*CHI:\tword@s .\n@End\n";
    let input2 = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|test|CHI|||||Child|||\n*CHI:\tword@s?\n@End\n";

    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;

    let result1 = parser.parse_chat_file(input1);
    println!("Input1 result: {:?}", result1);
    let file1 = result1.map_err(|_| {
        TestError::Failure("Should parse with whitespace after language marker".to_string())
    })?;
    if file1.utterances().count() != 1 {
        return Err(TestError::Failure(
            "Should have one utterance for input1".to_string(),
        ));
    }

    let result2 = parser.parse_chat_file(input2);
    println!("Input2 result: {:?}", result2);
    if let Ok(file) = &result2 {
        let count = file.utterances().count();
        if count == 0 {
            println!("ERROR: No utterances parsed for input2!");
        }
    }
    let file2 = result2.map_err(|_| {
        TestError::Failure("Should parse without whitespace after language marker".to_string())
    })?;
    if file2.utterances().count() != 1 {
        return Err(TestError::Failure(
            "Should have one utterance for input2".to_string(),
        ));
    }
    Ok(())
}

/// Tests simplified word then terminator.
#[test]
fn test_simplified_word_then_terminator() -> Result<(), TestError> {
    // Simplified test: just word body + terminator

    // Simulate word body parser (any text not containing forbidden chars)
    let word_body = none_of(".!?,;:^()[]{}\\<>@$*%\" \t\n\r&")
        .repeated()
        .at_least(1)
        .to_slice();

    // Simulate terminator parser
    let terminator = just::<_, _, extra::Err<Simple<char>>>('?').to("QUESTION");

    // Simulate main tier structure: word + optional_ws + optional_terminator
    let ws = just::<_, _, extra::Err<Simple<char>>>(' ')
        .repeated()
        .at_least(1)
        .ignored();

    let parser = word_body
        .then(ws.or_not())
        .then(terminator.or_not())
        .map(|((word, _ws), term)| (word, term));

    // Test with whitespace
    let result1 = parser.parse("word ?").into_result();
    println!("'word ?' result: {:?}", result1);
    if result1.is_err() {
        return Err(TestError::Failure("Expected 'word ?' to parse".to_string()));
    }

    // Test without whitespace
    let result2 = parser.parse("word?").into_result();
    println!("'word?' result: {:?}", result2);
    if result2.is_err() {
        return Err(TestError::Failure("Expected 'word?' to parse".to_string()));
    }
    Ok(())
}
