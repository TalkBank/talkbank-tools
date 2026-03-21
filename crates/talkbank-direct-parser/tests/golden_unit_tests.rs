//! Legacy bootstrap-era fragment tests for DirectParser.
//!
//! This suite still compares several direct-parser fragment paths against
//! tree-sitter fragment behavior. That was useful while the direct parser was
//! being bootstrapped, but it is no longer a trustworthy long-term oracle for
//! fragment semantics or lenient/recovery behavior.
//!
//! ## Testing Strategy
//!
//! 1. **Legacy TreeSitter comparison**: Compare some fragment outputs to
//!    TreeSitterParser behavior
//! 2. **Feature-Level Testing**: Test individual DirectParser components (words, tiers, etc.)
//! 3. **Fast Feedback**: Each test runs in milliseconds, not seconds
//! 4. **Known limitation**: this does not establish the correct oracle for
//!    direct-parser leniency or recovery
//!
//! ## Usage
//!
//! ```bash
//! # Run all unit tests (fast!)
//! cargo test -p talkbank-direct-parser
//!
//! # Run specific feature test
//! cargo test -p talkbank-direct-parser test_word_with_compound
//!
//! # Run with verbose output
//! cargo test -p talkbank-direct-parser -- --nocapture
//! ```

use talkbank_direct_parser::DirectParser;
use talkbank_model::ChatParser;
use talkbank_model::ErrorCollector;
use talkbank_model::model::{SemanticEq, Word};
use talkbank_parser::TreeSitterParser;
use talkbank_parser_tests::test_error::TestError;

/// Constructs a TreeSitter parser used by the legacy comparison tests.
fn tree_sitter_parser() -> Result<TreeSitterParser, TestError> {
    TreeSitterParser::new().map_err(|err| TestError::ParserInit(err.to_string()))
}

/// Constructs the Direct parser under test.
fn direct_parser() -> Result<DirectParser, TestError> {
    DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))
}

/// Helper: Parse a word with TreeSitter for legacy comparison.
///
/// This creates a minimal CHAT file with the word in a main tier, parses it
/// with TreeSitterParser, and extracts the Word object for comparison output.
fn parse_word_with_treesitter(word_text: &str) -> Result<Word, TestError> {
    let tree_sitter = tree_sitter_parser()?;
    let errors = ErrorCollector::new();

    // Parse just the word using TreeSitter's ChatParser implementation
    let word = ChatParser::parse_word(&tree_sitter, word_text, 0, &errors).ok_or_else(|| {
        TestError::Failure(format!(
            "TreeSitterParser failed to parse word '{}'",
            word_text
        ))
    })?;

    if !errors.is_empty() {
        return Err(TestError::Failure(format!(
            "TreeSitterParser reported errors for golden input '{}': {:?}",
            word_text,
            errors.to_vec()
        )));
    }

    Ok(word)
}

/// Helper: Parse a main tier line with TreeSitter.
///
/// **IMPORTANT: Input convention**
/// The input should be a complete main tier line including speaker prefix: `*SPEAKER:\tcontent`
/// This is different from text tiers which expect content-only input.
fn parse_main_tier_with_treesitter(
    input: &str,
) -> Result<talkbank_model::model::MainTier, TestError> {
    let tree_sitter = tree_sitter_parser()?;
    let errors = ErrorCollector::new();

    // Use trait method syntax to call ChatParser methods
    let tier = ChatParser::parse_main_tier(&tree_sitter, input, 0, &errors).ok_or_else(|| {
        TestError::Failure(format!(
            "TreeSitterParser failed to parse main tier '{}'",
            input
        ))
    })?;

    if !errors.is_empty() {
        return Err(TestError::Failure(format!(
            "TreeSitterParser reported errors for golden input '{}': {:?}",
            input,
            errors.to_vec()
        )));
    }

    Ok(tier)
}

/// Helper: Parse a text tier (%com) with TreeSitter.
///
/// **IMPORTANT: Input convention - CONTENT ONLY**
/// The input should be the comment tier CONTENT WITHOUT the %com:\t prefix.
///
/// ✅ CORRECT:
/// ```text
/// "after transcribing, I realized\n\tthis means something"
/// ```
///
/// ❌ INCORRECT:
/// ```text
/// "%com:\tafter transcribing, I realized\n\tthis means something"
/// ```
///
/// **Rationale:** TreeSitterParser's wrapper_parse_tier() adds the prefix internally.
/// Including the prefix would create invalid CHAT format during parsing.
/// See: API_PREFIX_CONVENTIONS.md
fn parse_com_tier_with_treesitter(
    input: &str,
) -> Result<talkbank_model::model::ComTier, TestError> {
    let tree_sitter = tree_sitter_parser()?;
    let errors = ErrorCollector::new();

    // Use trait method syntax to call ChatParser methods
    let tier = ChatParser::parse_com_tier(&tree_sitter, input, 0, &errors).ok_or_else(|| {
        TestError::Failure(format!(
            "TreeSitterParser failed to parse com tier '{}'",
            input
        ))
    })?;

    if !errors.is_empty() {
        return Err(TestError::Failure(format!(
            "TreeSitterParser reported errors for golden input '{}': {:?}",
            input,
            errors.to_vec()
        )));
    }

    Ok(tier)
}

/// Parses mor tier with treesitter.
fn parse_mor_tier_with_treesitter(
    input: &str,
) -> Result<talkbank_model::model::MorTier, TestError> {
    let tree_sitter = tree_sitter_parser()?;
    let errors = ErrorCollector::new();
    let tier = ChatParser::parse_mor_tier(&tree_sitter, input, 0, &errors).ok_or_else(|| {
        TestError::Failure(format!(
            "TreeSitterParser failed to parse mor tier '{}'",
            input
        ))
    })?;

    if !errors.is_empty() {
        return Err(TestError::Failure(format!(
            "TreeSitterParser reported errors for golden input '{}': {:?}",
            input,
            errors.to_vec()
        )));
    }

    Ok(tier)
}

/// Parses pho tier with treesitter.
fn parse_pho_tier_with_treesitter(
    input: &str,
) -> Result<talkbank_model::model::PhoTier, TestError> {
    let tree_sitter = tree_sitter_parser()?;
    let errors = ErrorCollector::new();
    let tier = ChatParser::parse_pho_tier(&tree_sitter, input, 0, &errors).ok_or_else(|| {
        TestError::Failure(format!(
            "TreeSitterParser failed to parse pho tier '{}'",
            input
        ))
    })?;

    if !errors.is_empty() {
        return Err(TestError::Failure(format!(
            "TreeSitterParser reported errors for golden input '{}': {:?}",
            input,
            errors.to_vec()
        )));
    }

    Ok(tier)
}

/// Parses word with direct.
fn parse_word_with_direct(word_text: &str) -> Result<Word, TestError> {
    let direct = direct_parser()?;
    let errors = ErrorCollector::new();
    let word = direct.parse_word(word_text, 0, &errors).ok_or_else(|| {
        TestError::Failure(format!("DirectParser returned None for '{}'", word_text))
    })?;

    if !errors.is_empty() {
        return Err(TestError::Failure(format!(
            "DirectParser reported errors for '{}': {:?}",
            word_text,
            errors.to_vec()
        )));
    }

    Ok(word)
}

/// Parses main tier with direct.
fn parse_main_tier_with_direct(input: &str) -> Result<talkbank_model::model::MainTier, TestError> {
    let direct = direct_parser()?;
    let errors = ErrorCollector::new();
    let tier = direct
        .parse_main_tier(input, 0, &errors)
        .ok_or_else(|| TestError::Failure(format!("DirectParser returned None for '{}'", input)))?;

    if !errors.is_empty() {
        return Err(TestError::Failure(format!(
            "DirectParser reported errors for '{}': {:?}",
            input,
            errors.to_vec()
        )));
    }

    Ok(tier)
}

/// Parses com tier with direct.
fn parse_com_tier_with_direct(input: &str) -> Result<talkbank_model::model::ComTier, TestError> {
    let direct = direct_parser()?;
    let errors = ErrorCollector::new();
    let tier = direct
        .parse_com_tier(input, 0, &errors)
        .ok_or_else(|| TestError::Failure(format!("DirectParser returned None for '{}'", input)))?;

    if !errors.is_empty() {
        return Err(TestError::Failure(format!(
            "DirectParser reported errors for '{}': {:?}",
            input,
            errors.to_vec()
        )));
    }

    Ok(tier)
}

/// Parses mor tier with direct.
fn parse_mor_tier_with_direct(input: &str) -> Result<talkbank_model::model::MorTier, TestError> {
    let direct = direct_parser()?;
    let errors = ErrorCollector::new();
    let tier = direct
        .parse_mor_tier(input, 0, &errors)
        .ok_or_else(|| TestError::Failure(format!("DirectParser returned None for '{}'", input)))?;

    if !errors.is_empty() {
        return Err(TestError::Failure(format!(
            "DirectParser reported errors for '{}': {:?}",
            input,
            errors.to_vec()
        )));
    }

    Ok(tier)
}

/// Parses pho tier with direct.
fn parse_pho_tier_with_direct(input: &str) -> Result<talkbank_model::model::PhoTier, TestError> {
    let direct = direct_parser()?;
    let errors = ErrorCollector::new();
    let tier = direct
        .parse_pho_tier(input, 0, &errors)
        .ok_or_else(|| TestError::Failure(format!("DirectParser returned None for '{}'", input)))?;

    if !errors.is_empty() {
        return Err(TestError::Failure(format!(
            "DirectParser reported errors for '{}': {:?}",
            input,
            errors.to_vec()
        )));
    }

    Ok(tier)
}

/// Asserts Direct and TreeSitter produce semantically equivalent `Word` values.
fn assert_word_matches(input: &str) -> Result<(), TestError> {
    let expected = parse_word_with_treesitter(input)?;
    let actual = parse_word_with_direct(input)?;

    if !actual.semantic_eq(&expected) {
        return Err(TestError::Failure(format!(
            "DirectParser diverged from TreeSitter on word '{}'
Expected: {:#?}
Actual: {:#?}",
            input, expected, actual
        )));
    }

    Ok(())
}

/// Asserts Direct and TreeSitter produce semantically equivalent main tiers.
fn assert_main_tier_matches(input: &str) -> Result<(), TestError> {
    let expected = parse_main_tier_with_treesitter(input)?;
    let actual = parse_main_tier_with_direct(input)?;

    if !actual.semantic_eq(&expected) {
        return Err(TestError::Failure(format!(
            "DirectParser diverged from TreeSitter on main tier '{}'
Expected: {:#?}
Actual: {:#?}",
            input, expected, actual
        )));
    }

    Ok(())
}

/// Asserts Direct and TreeSitter produce semantically equivalent `%com` tiers.
fn assert_com_tier_matches(input: &str) -> Result<(), TestError> {
    let expected = parse_com_tier_with_treesitter(input)?;
    let actual = parse_com_tier_with_direct(input)?;

    if !actual.semantic_eq(&expected) {
        return Err(TestError::Failure(format!(
            "DirectParser diverged from TreeSitter on com tier '{}'
Expected: {:#?}
Actual: {:#?}",
            input, expected, actual
        )));
    }

    Ok(())
}

/// Asserts Direct and TreeSitter produce semantically equivalent `%mor` tiers.
fn assert_mor_tier_matches(input: &str) -> Result<(), TestError> {
    let expected = parse_mor_tier_with_treesitter(input)?;
    let actual = parse_mor_tier_with_direct(input)?;

    if !actual.semantic_eq(&expected) {
        return Err(TestError::Failure(format!(
            "DirectParser diverged from TreeSitter on mor tier '{}'
Expected: {:#?}
Actual: {:#?}",
            input, expected, actual
        )));
    }

    Ok(())
}

/// Asserts Direct and TreeSitter produce semantically equivalent `%pho` tiers.
fn assert_pho_tier_matches(input: &str) -> Result<(), TestError> {
    let expected = parse_pho_tier_with_treesitter(input)?;
    let actual = parse_pho_tier_with_direct(input)?;

    if !actual.semantic_eq(&expected) {
        return Err(TestError::Failure(format!(
            "DirectParser diverged from TreeSitter on pho tier '{}'
Expected: {:#?}
Actual: {:#?}",
            input, expected, actual
        )));
    }

    Ok(())
}

// =============================================================================
// Word-Level Tests (Compound Words, Overlap Points, Stress, CA Elements)
// =============================================================================

/// Tests word simple.
#[test]
fn test_word_simple() -> Result<(), TestError> {
    let input = "hello";
    assert_word_matches(input)
}

/// Tests word with compound.
#[test]
fn test_word_with_compound() -> Result<(), TestError> {
    let input = "wai4+yu3";
    assert_word_matches(input)
}

/// Tests word with overlap marker open.
#[test]
fn test_word_with_overlap_marker_open() -> Result<(), TestError> {
    let input = "hello⌈";
    assert_word_matches(input)
}

/// Tests word with overlap marker close.
#[test]
fn test_word_with_overlap_marker_close() -> Result<(), TestError> {
    let input = "⌉world";
    assert_word_matches(input)
}

/// Tests word with stress primary.
#[test]
fn test_word_with_stress_primary() -> Result<(), TestError> {
    let input = "ˈstress";
    assert_word_matches(input)
}

/// Tests word with ca elements.
#[test]
fn test_word_with_ca_elements() -> Result<(), TestError> {
    let input = "hel∾lo"; // ∾ is constriction (U+223E), a valid CA element
    assert_word_matches(input)
}

/// Tests word with lengthening.
#[test]
fn test_word_with_lengthening() -> Result<(), TestError> {
    let input = "hello:";
    assert_word_matches(input)
}

/// Tests word with shortening.
#[test]
fn test_word_with_shortening() -> Result<(), TestError> {
    let input = "goin(g)";
    assert_word_matches(input)
}

/// Tests word complex combination.
#[test]
fn test_word_complex_combination() -> Result<(), TestError> {
    let input = "ˈhel⌈lo+wor⌉ld:";
    assert_word_matches(input)
}

/// Tests word midword ca element.
#[test]
fn test_word_midword_ca_element() -> Result<(), TestError> {
    let input = "hard⁑ening";
    let expected = parse_word_with_treesitter(input)?;

    println!("\n=== TreeSitter parse of '{}' ===", input);
    println!("raw_text: {:?}", expected.raw_text());
    println!("cleaned_text: {:?}", expected.cleaned_text());
    println!("content: {:#?}", expected.content);

    let actual = parse_word_with_direct(input)?;
    println!("\n=== DirectParser parse of '{}' ===", input);
    println!("raw_text: {:?}", actual.raw_text());
    println!("cleaned_text: {:?}", actual.cleaned_text());
    println!("content: {:#?}", actual.content);

    if !actual.semantic_eq(&expected) {
        return Err(TestError::Failure(format!(
            "DirectParser diverged from TreeSitter on midword CA element '{}'
Expected: {:#?}
Actual: {:#?}",
            input, expected, actual
        )));
    }
    Ok(())
}

/// Tests word midword overlap.
#[test]
fn test_word_midword_overlap() -> Result<(), TestError> {
    let input = "x⌈xx⌉";
    let expected = parse_word_with_treesitter(input)?;

    println!("\n=== TreeSitter parse of '{}' ===", input);
    println!("raw_text: {:?}", expected.raw_text());
    println!("cleaned_text: {:?}", expected.cleaned_text());
    println!("content: {:#?}", expected.content);

    let actual = parse_word_with_direct(input)?;
    println!("\n=== DirectParser parse of '{}' ===", input);
    println!("raw_text: {:?}", actual.raw_text());
    println!("cleaned_text: {:?}", actual.cleaned_text());
    println!("content: {:#?}", actual.content);

    if !actual.semantic_eq(&expected) {
        return Err(TestError::Failure(format!(
            "DirectParser diverged from TreeSitter on midword overlap '{}'
Expected: {:#?}
Actual: {:#?}",
            input, expected, actual
        )));
    }
    Ok(())
}

/// Tests word paired delimiter lower.
#[test]
fn test_word_paired_delimiter_lower() -> Result<(), TestError> {
    let input = "▁lower▁";
    let expected = parse_word_with_treesitter(input)?;

    println!("\n=== TreeSitter parse of '{}' ===", input);
    println!("raw_text: {:?}", expected.raw_text());
    println!("cleaned_text: {:?}", expected.cleaned_text());
    println!("content: {:#?}", expected.content);

    let actual = parse_word_with_direct(input)?;
    println!("\n=== DirectParser parse of '{}' ===", input);
    println!("raw_text: {:?}", actual.raw_text());
    println!("cleaned_text: {:?}", actual.cleaned_text());
    println!("content: {:#?}", actual.content);

    if !actual.semantic_eq(&expected) {
        return Err(TestError::Failure(format!(
            "DirectParser diverged from TreeSitter on paired delimiter '{}'
Expected: {:#?}
Actual: {:#?}",
            input, expected, actual
        )));
    }
    Ok(())
}

/// Tests word paired delimiter lowpitch.
#[test]
fn test_word_paired_delimiter_lowpitch() -> Result<(), TestError> {
    let input = "▁lowpitch▁";
    let expected = parse_word_with_treesitter(input)?;

    println!("\n=== TreeSitter parse of '{}' ===", input);
    println!("raw_text: {:?}", expected.raw_text());
    println!("cleaned_text: {:?}", expected.cleaned_text());
    println!("content: {:#?}", expected.content);

    let actual = parse_word_with_direct(input)?;
    println!("\n=== DirectParser parse of '{}' ===", input);
    println!("raw_text: {:?}", actual.raw_text());
    println!("cleaned_text: {:?}", actual.cleaned_text());
    println!("content: {:#?}", actual.content);

    if !actual.semantic_eq(&expected) {
        return Err(TestError::Failure(format!(
            "DirectParser diverged from TreeSitter on paired delimiter '{}'
Expected: {:#?}
Actual: {:#?}",
            input, expected, actual
        )));
    }
    Ok(())
}

/// Tests word pos tag simple.
#[test]
fn test_word_pos_tag_simple() -> Result<(), TestError> {
    let input = "bar$n";
    let expected = parse_word_with_treesitter(input)?;

    println!("\n=== TreeSitter parse of '{}' ===", input);
    println!("raw_text: {:?}", expected.raw_text());
    println!("cleaned_text: {:?}", expected.cleaned_text());
    println!("part_of_speech: {:?}", expected.part_of_speech);
    println!("content: {:#?}", expected.content);

    let actual = parse_word_with_direct(input)?;
    println!("\n=== DirectParser parse of '{}' ===", input);
    println!("raw_text: {:?}", actual.raw_text());
    println!("cleaned_text: {:?}", actual.cleaned_text());
    println!("part_of_speech: {:?}", actual.part_of_speech);
    println!("content: {:#?}", actual.content);

    if !actual.semantic_eq(&expected) {
        return Err(TestError::Failure(format!(
            "DirectParser diverged from TreeSitter on POS tag '{}'
Expected: {:#?}
Actual: {:#?}",
            input, expected, actual
        )));
    }
    Ok(())
}

/// Tests word pos tag complex.
#[test]
fn test_word_pos_tag_complex() -> Result<(), TestError> {
    let input = "bar$n:m:l";
    let expected = parse_word_with_treesitter(input)?;

    println!("\n=== TreeSitter parse of '{}' ===", input);
    println!("raw_text: {:?}", expected.raw_text());
    println!("cleaned_text: {:?}", expected.cleaned_text());
    println!("part_of_speech: {:?}", expected.part_of_speech);
    println!("content: {:#?}", expected.content);

    let actual = parse_word_with_direct(input)?;
    println!("\n=== DirectParser parse of '{}' ===", input);
    println!("raw_text: {:?}", actual.raw_text());
    println!("cleaned_text: {:?}", actual.cleaned_text());
    println!("part_of_speech: {:?}", actual.part_of_speech);
    println!("content: {:#?}", actual.content);

    if !actual.semantic_eq(&expected) {
        return Err(TestError::Failure(format!(
            "DirectParser diverged from TreeSitter on POS tag '{}'
Expected: {:#?}
Actual: {:#?}",
            input, expected, actual
        )));
    }
    Ok(())
}

// =============================================================================
// Golden Words Corpus Tests
// =============================================================================

/// Test all golden words from the reference corpus (769 words).
///
/// This test provides comprehensive coverage by testing every unique word
/// extracted from corpus/reference/ against TreeSitterParser as golden output.
#[test]
fn test_all_golden_words() -> Result<(), TestError> {
    use talkbank_parser_tests::golden::golden_words;

    let tree_sitter = tree_sitter_parser()?;
    let direct = direct_parser()?;

    let words = golden_words();
    let mut failures = Vec::new();

    for word_text in words {
        let ts_errors = ErrorCollector::new();
        let expected = ChatParser::parse_word(&tree_sitter, word_text, 0, &ts_errors);

        // Skip if TreeSitter can't parse it (not a valid golden word)
        if !ts_errors.is_empty() || expected.is_none() {
            continue;
        }
        let expected = expected.ok_or_else(|| {
            TestError::Failure(format!(
                "TreeSitterParser returned None for golden word '{}'",
                word_text
            ))
        })?;

        let direct_errors = ErrorCollector::new();
        let actual = direct.parse_word(word_text, 0, &direct_errors);

        if !direct_errors.is_empty() {
            failures.push(format!(
                "Word '{}': DirectParser reported errors: {:?}",
                word_text,
                direct_errors.to_vec()
            ));
            continue;
        }

        let actual = actual.ok_or_else(|| {
            TestError::Failure(format!("DirectParser returned None for '{}'", word_text))
        })?;
        if !actual.semantic_eq(&expected) {
            failures.push(format!(
                "Word '{}': Semantic mismatch
Expected: {:#?}
Actual: {:#?}",
                word_text, expected, actual
            ));
        }
    }

    if !failures.is_empty() {
        return Err(TestError::Failure(format!(
            "DirectParser diverged from TreeSitter on {} golden words:\n{}",
            failures.len(),
            failures.join("\n\n")
        )));
    }

    Ok(())
}

// =============================================================================
// Phase 1 TDD Tests: Parse Failures Fixed
// =============================================================================

/// Test standalone parenthesized words like (parens) for CA uncertain transcription.
/// These are complete words by themselves, not shortenings within words.
#[test]
fn test_word_standalone_parens_uncertain() -> Result<(), TestError> {
    let input = "(parens)";
    assert_word_matches(input)
}

/// Test word-initial shortening like (t)a where the shortening is at the start.
#[test]
fn test_word_initial_shortening() -> Result<(), TestError> {
    let input = "(t)a";
    assert_word_matches(input)
}

/// Test word with shortening followed by underscore connector: (there)_now
#[test]
fn test_word_shortening_with_underscore() -> Result<(), TestError> {
    let input = "(there)_now";
    assert_word_matches(input)
}

// =============================================================================
// Phase 2: Semantic Mismatch Tests
// =============================================================================

/// Test CA continuation marker in scoped annotation: what [^c] why
/// The [^c] should be recognized as a scoped annotation with content "^c"
#[test]
fn test_ca_continuation_marker() -> Result<(), TestError> {
    let input = "*CHI:\twhat [^c] why .";
    assert_main_tier_matches(input)
}

/// Test text tier with inline bullets: foo 2061689_2062652 bar
/// Bullets should be parsed as separate segments within BulletContent
#[test]
fn test_text_tier_with_bullets() -> Result<(), TestError> {
    let input = "foo 2061689_2062652 bar";
    assert_com_tier_matches(input)
}

// =============================================================================
// CA Feature Tests
// =============================================================================

/// Test CA continuation marker [^c] as a separator in main tier content
/// This is from c.cha - a simple test file with just the [^c] marker
#[test]
fn test_ca_continuation_separator() -> Result<(), TestError> {
    let input = "*CHI:\twhat [^c] why now ?";
    assert_main_tier_matches(input)
}

/// Test CA blocked segment marker: rub-b-b≠bber
#[test]
fn test_ca_blocked_segment() -> Result<(), TestError> {
    let input = "*CHI:\tblocked segments rub-b-b≠bber .";
    assert_main_tier_matches(input)
}

// =============================================================================
// Mor Tier Tests
// =============================================================================

/// Test form markers without space: a@l (letter form marker)
#[test]
fn test_form_marker_no_space() -> Result<(), TestError> {
    let input = "a@l";
    assert_word_matches(input)
}

/// Test mor tier with UD-style POS and translation alternatives
#[test]
fn test_mor_translation_alternatives() -> Result<(), TestError> {
    let input = "det|mi-Def-Prs-Sing-S noun|música-Fem pron|mi-Prs-Nom-S2 .";
    assert_mor_tier_matches(input)
}

/// Test multi-line comment tier with continuation (content-only input)
#[test]
fn test_multiline_comment_tier() -> Result<(), TestError> {
    // %com tier spanning multiple lines (content-only, without %com:\t prefix)
    let input = "after transcribing the entire file, I realized that pʊʃ and\n\tsimilar sounding things might mean \"open this .\" Things that sound\n\tlike əˈmaɪ or ʊˈmaɪ seem to connote desire roughly, \"I want\n\tMommy to\" or \"I want you to\" or something like that: this is what\n\tCHI usually says when he's asking his mom to do something for him .\n\tI've transcribed it as \"Mommy\" .";

    assert_com_tier_matches(input)
}

/// Test simple pho tier parsing
#[test]
fn test_pho_tier_simple() -> Result<(), TestError> {
    let input = "hɛˈloʊ ðɛr .";
    assert_pho_tier_matches(input)
}
