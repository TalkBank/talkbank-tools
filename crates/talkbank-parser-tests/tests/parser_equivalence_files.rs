//! Per-file parser equivalence tests.
//!
//! Each `.cha` file in the reference corpus becomes its own `#[test]`, enabling:
//! - Per-file parallelism via nextest
//! - Individual test filtering (`cargo nextest run -E 'test(filename)'`)
//! - Failure isolation (one bad file doesn't block others)
//!
//! ## Usage
//!
//! ```bash
//! # Run all parser equivalence tests (nextest compatible)
//! cargo nextest run -p talkbank-parser-tests -E 'test(parser_equivalence)'
//!
//! # Run a single file's test
//! cargo nextest run -p talkbank-parser-tests -E 'test(some_filename)'
//!
//! # Show output on failure
//! cargo nextest run -p talkbank-parser-tests --no-capture
//! ```

use std::panic;
use std::path::PathBuf;

use rstest::rstest;
use talkbank_direct_parser::DirectParser;
use talkbank_model::model::SemanticEq;
use talkbank_parser::TreeSitterParser;

/// Files containing constructs the direct parser does not support.
///
/// These are skipped from cross-parser equivalence — tested with tree-sitter only.
const DIRECT_PARSER_SKIP: &[&str] = &[];

/// Test that TreeSitterParser and DirectParser produce semantically equivalent
/// ChatFile models for a single reference corpus file.
///
/// ## Failure Cases
///
/// - **DirectParser fails/panics, TreeSitter succeeds**: DirectParser bug (FAIL)
/// - **Both succeed but differ**: Semantic equivalence bug (FAIL)
/// - **TreeSitter fails**: Not testing DirectParser strictness (PASS)
/// - **Both fail**: Nothing to compare (PASS)
/// - **File in DIRECT_PARSER_SKIP**: Skipped (PASS)
#[rstest]
fn parser_equivalence(#[files("../../corpus/reference/**/*.cha")] path: PathBuf) {
    // Skip files with known direct parser limitations
    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        if DIRECT_PARSER_SKIP.contains(&stem) {
            return;
        }
    }

    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));

    let ts = TreeSitterParser::new().expect("TreeSitterParser init");
    let direct = DirectParser::new().expect("DirectParser init");

    let ts_result = ts.parse_chat_file(&content);

    // Catch panics from DirectParser (safety net for experimental parser)
    let direct_result = panic::catch_unwind(|| direct.parse_chat_file(&content));

    match (ts_result, direct_result) {
        // Both succeeded — check semantic equivalence
        (Ok(ts_file), Ok(Ok(direct_file))) => {
            assert!(
                ts_file.semantic_eq(&direct_file),
                "Semantic mismatch: parsers produced different models for {}",
                path.display()
            );
        }
        // TreeSitter succeeded, DirectParser returned error
        (Ok(_), Ok(Err(e))) => {
            panic!(
                "DirectParser failed but TreeSitter succeeded for {}: {:?}",
                path.display(),
                e
            );
        }
        // TreeSitter succeeded, DirectParser panicked
        (Ok(_), Err(panic_err)) => {
            let msg = panic_err
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic_err.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown panic".to_string());
            panic!(
                "DirectParser panicked but TreeSitter succeeded for {}: {}",
                path.display(),
                msg
            );
        }
        // TreeSitter failed — reference corpus files MUST parse successfully
        (Err(e), _) => {
            panic!(
                "TreeSitter failed to parse reference corpus file {}: {:?}",
                path.display(),
                e
            );
        }
    }
}
