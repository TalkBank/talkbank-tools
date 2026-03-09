//! Snapshot testing infrastructure for parser golden tests.
//!
//! Provides utilities for comparing parser outputs semantically and generating
//! insta snapshots for regression testing.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

use crate::test_error::TestError;
use talkbank_model::ErrorCollector;
use talkbank_model::model::SemanticEq;
use talkbank_model::{ChatParser, ParseOutcome};

/// Compare outputs from two parsers and snapshot the tree-sitter result.
///
/// This function:
/// 1. Parses input with both parsers
/// 2. Verifies semantic equality
/// 3. Snapshots the tree-sitter result for regression testing
///
/// # Panics
/// If the two parsers produce semantically different results.
pub fn compare_parsers_for_word(
    tree_sitter: &impl ChatParser,
    direct: &impl ChatParser,
    input: &str,
) -> Result<serde_json::Value, TestError> {
    let errors_ts = ErrorCollector::new();
    let errors_direct = ErrorCollector::new();

    let ts_result = ChatParser::parse_word(tree_sitter, input, 0, &errors_ts);
    let direct_result = ChatParser::parse_word(direct, input, 0, &errors_direct);

    match (ts_result, direct_result) {
        (ParseOutcome::Parsed(ts_word), ParseOutcome::Parsed(direct_word)) => {
            if !ts_word.semantic_eq(&direct_word) {
                return Err(TestError::Failure(format!(
                    "Parsers differ for word '{}'\nTree-sitter:\n{:#?}
Direct:\n{:#?}",
                    input, ts_word, direct_word
                )));
            }
            Ok(serde_json::to_value(&ts_word)?)
        }
        (ParseOutcome::Parsed(ts_word), ParseOutcome::Rejected) => {
            Err(TestError::Failure(format!(
                "Direct parser failed for word '{}' but tree-sitter succeeded\nTree-sitter result:\n{:#?}",
                input, ts_word
            )))
        }
        (ParseOutcome::Rejected, ParseOutcome::Parsed(direct_word)) => {
            Err(TestError::Failure(format!(
                "Tree-sitter failed for word '{}' but direct parser succeeded\nDirect result:\n{:#?}",
                input, direct_word
            )))
        }
        (ParseOutcome::Rejected, ParseOutcome::Rejected) => Err(TestError::Failure(format!(
            "Both parsers failed for word '{}'",
            input
        ))),
    }
}

/// Compare outputs from two parsers for a complete file and snapshot the tree-sitter result.
pub fn compare_parsers_for_file(
    tree_sitter: &impl ChatParser,
    direct: &impl ChatParser,
    content: &str,
) -> Result<serde_json::Value, TestError> {
    let errors_ts = ErrorCollector::new();
    let errors_direct = ErrorCollector::new();

    let ts_result = ChatParser::parse_chat_file(tree_sitter, content, 0, &errors_ts);
    let direct_result = ChatParser::parse_chat_file(direct, content, 0, &errors_direct);

    if !ts_result.semantic_eq(&direct_result) {
        return Err(TestError::Failure(format!(
            "Parsers differ for file\nTree-sitter:\n{:#?}
Direct:\n{:#?}",
            ts_result, direct_result
        )));
    }

    let Some(ts_file) = ts_result.into_option() else {
        return Err(TestError::Failure(
            "Both parsers rejected file input".to_string(),
        ));
    };

    Ok(serde_json::to_value(&ts_file)?)
}

/// Test helper: parametrized comparison for words from spec examples.
///
/// # Example
/// ```ignore
/// #[test]
/// fn spec_word_basic_hello() {
///     spec_word_test("hello");
/// }
/// ```
pub fn spec_word_test(input: &str) -> Result<(), TestError> {
    let tree_sitter = talkbank_parser::TreeSitterParser::new()
        .map_err(|err| TestError::ParserInit(err.to_string()))?;
    let direct = talkbank_direct_parser::DirectParser::new().map_err(TestError::ParserInit)?;

    let snapshot = compare_parsers_for_word(&tree_sitter, &direct, input)?;
    insta::assert_json_snapshot!(snapshot);
    Ok(())
}

/// Files containing constructs the direct parser does not support.
///
/// These files are tested with tree-sitter only (no cross-parser comparison).
/// The direct parser is experimental and partial — unsupported headers/lines
/// and certain other constructs are not implemented.
const DIRECT_PARSER_SKIP_FILES: &[&str] = &[];

/// Test helper: parametrized comparison for files from reference corpus.
///
/// # Example
/// ```ignore
/// #[test]
/// fn reference_corpus_abc_001() {
///     reference_file_test("corpus/reference/ABC_001.cha");
/// }
/// ```
pub fn reference_file_test(path: &str) -> Result<(), TestError> {
    // Resolve path relative to repo root if it doesn't exist as-is
    let full_path = if std::path::Path::new(path).exists() {
        path.to_string()
    } else {
        // CARGO_MANIFEST_DIR is crates/talkbank-parser-tests
        // corpus is at repo root, so go up two levels (../.. from crate dir)
        let crate_dir = match std::env::var("CARGO_MANIFEST_DIR") {
            Ok(dir) => dir,
            Err(_err) => ".".to_string(),
        };
        let repo_root = std::path::Path::new(&crate_dir).join("../..").join(path);
        repo_root.to_string_lossy().to_string()
    };

    let content = std::fs::read_to_string(&full_path)?;

    // Use filename as snapshot suffix to make each test case unique
    let filename = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        // DEFAULT: If the file stem is unavailable, use a stable snapshot name.
        .unwrap_or("unknown");

    let skip_direct = DIRECT_PARSER_SKIP_FILES.contains(&filename);

    let snapshot = if skip_direct {
        // Tree-sitter only — direct parser does not support these constructs
        bootstrap_file_snapshot_from_content(&content)?
    } else {
        let tree_sitter = talkbank_parser::TreeSitterParser::new()
            .map_err(|err| TestError::ParserInit(err.to_string()))?;
        let direct = talkbank_direct_parser::DirectParser::new().map_err(TestError::ParserInit)?;
        compare_parsers_for_file(&tree_sitter, &direct, &content)?
    };

    insta::assert_json_snapshot!(filename, snapshot);
    Ok(())
}

/// Test helper: parse with tree-sitter and snapshot (for initial bootstrap).
pub fn bootstrap_word_snapshot(input: &str) -> Result<serde_json::Value, TestError> {
    let tree_sitter = talkbank_parser::TreeSitterParser::new()
        .map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors = ErrorCollector::new();

    let result = ChatParser::parse_word(&tree_sitter, input, 0, &errors);
    match result {
        ParseOutcome::Parsed(word) => Ok(serde_json::to_value(&word)?),
        ParseOutcome::Rejected => Err(TestError::Failure(format!(
            "Tree-sitter failed to parse word '{}'",
            input
        ))),
    }
}

/// Parse content with tree-sitter only and return a snapshot value.
///
/// Used for files that contain constructs the direct parser does not support.
fn bootstrap_file_snapshot_from_content(content: &str) -> Result<serde_json::Value, TestError> {
    let tree_sitter = talkbank_parser::TreeSitterParser::new()
        .map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors = ErrorCollector::new();

    match ChatParser::parse_chat_file(&tree_sitter, content, 0, &errors) {
        ParseOutcome::Parsed(file) => Ok(serde_json::to_value(&file)?),
        ParseOutcome::Rejected => Err(TestError::Failure(
            "Tree-sitter failed to parse file".to_string(),
        )),
    }
}

/// Test helper: parse file with tree-sitter and snapshot (for initial bootstrap).
pub fn bootstrap_file_snapshot(path: &str) -> Result<serde_json::Value, TestError> {
    let content = std::fs::read_to_string(path)?;
    let tree_sitter = talkbank_parser::TreeSitterParser::new()
        .map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors = ErrorCollector::new();

    match ChatParser::parse_chat_file(&tree_sitter, &content, 0, &errors) {
        ParseOutcome::Parsed(file) => Ok(serde_json::to_value(&file)?),
        ParseOutcome::Rejected => Err(TestError::Failure(
            "Tree-sitter failed to parse file".to_string(),
        )),
    }
}
