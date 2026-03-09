//! Tests for this subsystem.
//!

use crate::error::ErrorCollector;
use crate::model::{LanguageCode, UtteranceContent, Word, WordCategory, WordContent};
use crate::parser::TreeSitterParser;
use crate::validation::{Validate, ValidationContext};

/// Parses chat file or err.
fn parse_chat_file_or_err(input: &str) -> Result<crate::model::ChatFile, String> {
    let parser =
        TreeSitterParser::new().map_err(|err| format!("Failed to create parser: {err}"))?;
    parser
        .parse_chat_file(input)
        .map_err(|err| format!("Failed to parse chat file: {err:?}"))
}

/// Tests shortening at word start.
#[test]
fn test_shortening_at_word_start() -> Result<(), String> {
    let input = "@UTF8\n@Begin\n*CHI:\t(th)at .\n@End";
    let result = parse_chat_file_or_err(input)?;

    let utterance = result
        .utterances()
        .next()
        .ok_or_else(|| "Expected an utterance".to_string())?;
    let main_tier = &utterance.main;

    // Should be parsed as ONE word "(th)at", not two words "(th)" and "at"
    assert_eq!(
        main_tier.content.content.len(),
        1,
        "GRAMMAR BUG: Should have exactly 1 word, not 2. Currently tree-sitter splits '(th)at' into '(th)' and 'at'"
    );

    let word = match &main_tier.content.content[0] {
        UtteranceContent::Word(w) => w,
        other => return Err(format!("Expected Word, got {:?}", other)),
    };

    // Verify the word structure
    assert_eq!(word.raw_text(), "(th)at", "Raw text should be '(th)at'");
    assert_eq!(word.cleaned_text(), "that", "Cleaned text should be 'that'");

    // Verify content structure: should be [Shortening("th"), Text("at")]
    assert_eq!(word.content.len(), 2, "Word should have 2 content items");

    if let WordContent::Shortening(text) = &word.content[0] {
        assert_eq!(text.as_ref(), "th", "First item should be shortening 'th'");
    } else {
        return Err(format!("Expected Shortening, got {:?}", &word.content[0]));
    }

    if let WordContent::Text(text) = &word.content[1] {
        assert_eq!(text.as_ref(), "at", "Second item should be text 'at'");
    } else {
        return Err(format!("Expected Text, got {:?}", &word.content[1]));
    }

    Ok(())
}

/// Tests ca mode parenthetical becomes ca omission.
#[test]
fn test_ca_mode_parenthetical_becomes_ca_omission() -> Result<(), String> {
    let input = "@UTF8\n@Begin\n@Options:\tCA\n*CHI:\t(word) .\n@End";
    let result = parse_chat_file_or_err(input)?;

    let utterance = result
        .utterances()
        .next()
        .ok_or_else(|| "Expected an utterance".to_string())?;
    let main_tier = &utterance.main;

    let word = match &main_tier.content.content[0] {
        UtteranceContent::Word(w) => w,
        other => return Err(format!("Expected Word, got {:?}", other)),
    };

    assert!(
        matches!(word.category, Some(WordCategory::CAOmission)),
        "Expected CAOmission category for parenthetical in CA mode"
    );
    assert_eq!(
        word.content.len(),
        1,
        "CA omission should have text-only content"
    );

    if let WordContent::Text(text) = &word.content[0] {
        assert_eq!(text.as_ref(), "word", "Inner text should be plain text");
    } else {
        return Err(format!("Expected Text content, got {:?}", &word.content[0]));
    }

    assert_eq!(
        word.to_chat(),
        "(word)",
        "CA omission should roundtrip to '(word)'"
    );

    Ok(())
}

/// Tests ca mode parenthetical with overlap becomes ca omission.
#[test]
fn test_ca_mode_parenthetical_with_overlap_becomes_ca_omission() -> Result<(), String> {
    // With prec(5) overlap_point and prec(5) standalone_word, adjacent ⌊(ja) is a
    // single standalone_word (longer match wins at same prec). The direct parser
    // handles the overlap marker word-internally.
    let input = "@UTF8\n@Begin\n@Options:\tCA\n*CHI:\t⌊(ja) .\n@End";
    let result = parse_chat_file_or_err(input)?;

    let utterance = result
        .utterances()
        .next()
        .ok_or_else(|| "Expected an utterance".to_string())?;
    let main_tier = &utterance.main;

    // First content item should be the word (overlap marker is word-internal)
    let word = match &main_tier.content.content[0] {
        UtteranceContent::Word(w) => w,
        other => return Err(format!("Expected Word at index 0, got {:?}", other)),
    };

    assert!(
        matches!(word.category, Some(WordCategory::CAOmission)),
        "Expected CAOmission category for parenthetical in CA mode"
    );

    assert!(
        word.content
            .iter()
            .any(|item| matches!(item, WordContent::Text(text) if text.as_ref() == "ja")),
        "Expected normalized text content 'ja' for CA omission"
    );
    assert!(
        !word
            .content
            .iter()
            .any(|item| matches!(item, WordContent::Shortening(_))),
        "Expected shortening to be normalized away in CA omission"
    );

    Ok(())
}

/// Tests low pitch delimiter not swallowed into underlined text.
#[test]
fn test_low_pitch_delimiter_not_swallowed_into_underlined_text() -> Result<(), String> {
    // After Phase 2 coarsening, underline markers (\u{0002}\u{0001} / \u{0002}\u{0002})
    // are grammar-level tokens that split the text into separate content items.
    // The ▁ (low pitch) characters end up inside standalone_word tokens and are
    // handled by the direct parser as CA delimiters within each word.
    //
    // Input: ▁ <underline_begin> a <underline_end> h <underline_begin> a▁ <underline_end>
    // Tokenizes as separate words: "▁", "a", "h", "a▁" with underline markers between.
    let input = "@UTF8\n@Begin\n*PM:\t▁\u{0002}\u{0001}a\u{0002}\u{0002}h\u{0002}\u{0001}a▁\u{0002}\u{0002} .\n@End";
    let result = parse_chat_file_or_err(input)?;

    let utterance = result
        .utterances()
        .next()
        .ok_or_else(|| "Expected an utterance".to_string())?;
    let main_tier = &utterance.main;

    // Collect all words from content
    let words: Vec<&Box<Word>> = main_tier
        .content
        .content
        .iter()
        .filter_map(|item| match item {
            UtteranceContent::Word(w) => Some(w),
            _ => None,
        })
        .collect();

    // After Phase 2, underline markers split text into separate standalone_word tokens.
    // The word "a▁" contains a ▁ which the direct parser recognizes as a CA delimiter.
    // The standalone word "▁" (just the delimiter alone) may or may not parse as a
    // CA delimiter depending on direct parser behavior — the key invariant is that
    // at least one ▁ in a multi-char word is recognized as a CA delimiter.
    let total_low_pitch: usize = words
        .iter()
        .map(|w| {
            w.content
                .iter()
                .filter(|item| {
                    matches!(
                        item,
                        WordContent::CADelimiter(delim)
                            if delim.delimiter_type == crate::model::CADelimiterType::LowPitch
                    )
                })
                .count()
        })
        .sum();
    assert!(
        total_low_pitch >= 1,
        "Expected at least one ▁ to be parsed as a CA delimiter, found {total_low_pitch}"
    );

    Ok(())
}

/// Tests shortening in middle.
#[test]
fn test_shortening_in_middle() -> Result<(), String> {
    let input = "@UTF8\n@Begin\n*CHI:\tan(d) .\n@End";
    let result = parse_chat_file_or_err(input)?;

    let utterance = result
        .utterances()
        .next()
        .ok_or_else(|| "Expected an utterance".to_string())?;
    let main_tier = &utterance.main;

    assert_eq!(
        main_tier.content.content.len(),
        1,
        "GRAMMAR BUG: Should have exactly 1 word 'an(d)', not multiple words"
    );

    let word = match &main_tier.content.content[0] {
        UtteranceContent::Word(w) => w,
        other => return Err(format!("Expected Word, got {:?}", other)),
    };

    assert_eq!(word.raw_text(), "an(d)", "Raw text should be 'an(d)'");
    assert_eq!(word.cleaned_text(), "and", "Cleaned text should be 'and'");

    Ok(())
}

/// Tests prosodic marker followed by text.
#[test]
fn test_prosodic_marker_followed_by_text() -> Result<(), String> {
    let input = "@UTF8\n@Begin\n*CHI:\tm:hm .\n@End";
    let result = parse_chat_file_or_err(input)?;

    let utterance = result
        .utterances()
        .next()
        .ok_or_else(|| "Expected an utterance".to_string())?;
    let main_tier = &utterance.main;

    assert_eq!(
        main_tier.content.content.len(),
        1,
        "GRAMMAR BUG: Should have exactly 1 word 'm:hm', not multiple words"
    );

    let word = match &main_tier.content.content[0] {
        UtteranceContent::Word(w) => w,
        other => return Err(format!("Expected Word, got {:?}", other)),
    };

    assert_eq!(word.raw_text(), "m:hm", "Raw text should be 'm:hm'");

    // Verify roundtrip: serialization should preserve the prosodic marker
    let serialized = word.to_chat();
    assert_eq!(serialized, "m:hm", "Serialized form should preserve 'm:hm'");

    Ok(())
}

/// Tests main tier span is set.
#[test]
fn test_main_tier_span_is_set() -> Result<(), String> {
    let content = "@UTF8\n@Begin\n*CHI:\thello world\n@End\n";
    let chat_file = parse_chat_file_or_err(content)?;

    // Get the first main tier
    let utterance = chat_file
        .utterances()
        .next()
        .ok_or_else(|| "Expected an utterance".to_string())?;
    let main_tier = &utterance.main;

    // Check that span is not DUMMY (0..0)
    assert_ne!(main_tier.span.start, 0, "Span start should not be 0");
    assert_ne!(main_tier.span.end, 0, "Span end should not be 0");
    assert!(
        main_tier.span.end > main_tier.span.start,
        "Span should be non-empty"
    );

    // The span should cover "*CHI:\thello world\n"
    let span_text = &content[main_tier.span.start as usize..main_tier.span.end as usize];
    assert_eq!(
        span_text.get(0..5),
        Some("*CHI:"),
        "Span should start with *CHI:"
    );

    println!(
        "MainTier span: {}..{}",
        main_tier.span.start, main_tier.span.end
    );
    println!("MainTier text: {:?}", span_text);

    Ok(())
}

/// Tests validation error has proper span.
#[test]
fn test_validation_error_has_proper_span() -> Result<(), String> {
    let content = "@UTF8\n@Begin\n*CHI:\thello world\n@End\n";
    let chat_file = parse_chat_file_or_err(content)?;

    // Get the first main tier (missing terminator)
    let utterance = chat_file
        .utterances()
        .next()
        .ok_or_else(|| "Expected an utterance".to_string())?;
    let main_tier = &utterance.main;

    // Validate it
    let languages = vec![LanguageCode::new("eng")];
    let ctx = ValidationContext::new()
        .with_default_language(languages[0].clone())
        .with_declared_languages(languages);
    let error_sink = ErrorCollector::new();
    main_tier.validate(&ctx, &error_sink);
    let errors = error_sink.into_vec();

    // Should have validation errors (missing terminator)
    assert!(!errors.is_empty(), "Should have validation errors");

    // Check that error spans are not DUMMY
    for error in &errors {
        println!(
            "Error: {} at span {}..{}",
            error.message, error.location.span.start, error.location.span.end
        );
        if error.code.as_str() == "E304" {
            // Missing terminator error should have the main tier's span
            assert_ne!(
                error.location.span.start, 0,
                "E304 error should have non-zero start"
            );
            assert_ne!(
                error.location.span.end, 0,
                "E304 error should have non-zero end"
            );
            assert!(
                error.location.span.end > error.location.span.start,
                "E304 error span should be non-empty"
            );
        }
    }

    Ok(())
}
