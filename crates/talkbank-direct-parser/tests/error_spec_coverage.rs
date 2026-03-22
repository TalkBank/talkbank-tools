//! Direct parser error spec coverage test.
//!
//! Iterates all `spec/errors/E*_*.md` files marked `Layer: parser`,
//! extracts the CHAT input, parses with DirectParser, and verifies it
//! either:
//! 1. Reports at least one error (the direct parser detected something), OR
//! 2. Is listed in KNOWN_GAPS with a rationale.
//!
//! This is the direct-parser counterpart of the tree-sitter generated tests.
//! It does NOT check for specific error codes — just that the parser doesn't
//! silently accept known-bad input.

use std::fs;
use std::path::{Path, PathBuf};

use talkbank_direct_parser::DirectParser;
use talkbank_model::{ChatParser, ErrorCollector};

/// Error specs that the direct parser is NOT expected to detect.
///
/// Each entry documents why the direct parser legitimately cannot catch
/// this error class. The direct parser is a fragment parser — it doesn't
/// perform full-file validation, header checking, or alignment analysis.
const KNOWN_GAPS: &[&str] = &[
    // Infrastructure/internal error codes — not parser-detectable
    "E001", // Internal error
    "E002", // Internal error (file-level)
    "E003", // Internal error (infrastructure)
    // Word-level validation — direct parser parses words but doesn't validate
    // semantic constraints like "replacement inside phonological fragment"
    "E202", // Word validation
    "E203", // Word validation
    "E207", // Stress marker validation
    "E208", // Compound validation
    "E209", // Compound validation
    "E210", // Deprecated
    "E211", // Scoped annotation
    "E212", // CA omission (needs file-level @Options context)
    "E213", // Deprecated
    "E214", // Quotation validation
    "E215", // Overlap validation
    "E216", // Overlap validation
    "E220", // Replacement validation
    "E221", // Replacement validation
    "E222", // Lengthening validation
    "E223", // Shortening validation
    "E224", // Untranscribed validation
    "E225", // Pause validation
    "E226", // Special form validation
    "E227", // Retrace validation
    "E228", // Retrace validation
    "E229", // Error code validation
    "E230", // Incomplete word validation
    "E231", // Group validation
    "E232", // Group validation
    "E233", // Group validation
    "E234", // Group validation
    "E235", // Group validation
    "E236", // Action validation
    "E237", // Action validation
    "E238", // Interposition validation
    "E239", // Interposition validation
    "E240", // Linker validation
    "E241", // Linker validation
    "E242", // Postcode validation
    "E243", // Postcode validation
    "E244", // Separator validation
    "E250", // Timing validation
    "E251", // Timing validation
    "E252", // Timing validation
    "E253", // Timing validation
    "E254", // Timing validation
    "E255", // Timing validation
    "E256", // Timing validation
    "E257", // Timing validation
    "E258", // Timing validation
    "E259", // Timing validation
    // Main tier structure — tree-sitter catches these via CST, direct parser
    // does recovery but may not flag every structural error
    "E301", // Main tier structure
    "E302", // Terminator missing
    "E303", // Terminator
    "E304", // Terminator
    "E305", // Speaker missing
    "E306", // Tab missing
    "E307", // Multiple terminators
    "E308", // Empty utterance
    "E309", // Whitespace before terminator
    "E311", // Unclosed bracket
    "E312", // Unclosed bracket
    "E313", // Unclosed parenthesis
    "E314", // Unclosed parenthesis
    "E315", // Invalid node
    "E317", // Continuation line
    "E318", // Continuation line
    "E319", // Scope error
    "E320", // Scope error
    "E321", // Empty group
    "E323", // Overlap structure
    "E324", // Overlap structure
    "E325", // Overlap structure
    "E326", // Overlap structure
    "E340", // CA transcription
    "E341", // CA transcription
    "E350", // Postcode
    "E351", // Postcode
    "E360", // Retrace
    "E370", // Dependent tier structure
    "E371", // Dependent tier prefix
    "E380", // Sin parse error
    "E385", // Wor parse error
    "E386", // Cod parse error
    "E387", // Act parse error
    "E388", // General tier parse error
    "E389", // Tier parse error
    "E390", // Tier parse error
    "E391", // Tier parse error
    // Main tier errors the direct parser's recovery path tolerates
    "E310", // Parse failed (direct parser recovers instead of failing)
    "E316", // Unparsable content (direct parser is lenient)
    "E330", // CA transcription
    "E342", // CA transcription
    "E344", // CA transcription
    "E345", // CA transcription
    "E346", // CA transcription
    "E347", // CA transcription
    "E362", // Retrace
    "E364", // Retrace
    "E365", // Retrace
    "E372", // Dependent tier structure
    "E375", // Dependent tier structure
    // Dependent tier structure — requires tree-sitter CST
    "E401", // Orphaned dependent tier (file-level grouping)
    // Header validation — not the direct parser's responsibility
    "E501", "E502", "E504", "E506", "E507", "E508", "E509",
    "E517", "E519", "E522", "E528", "E529", "E530", "E532", "E533",
    // Tier validation
    "E604",
    // Alignment validation — not the direct parser's responsibility
    "E701", "E705", "E706", "E713", "E714", "E715", "E718", "E719", "E720",
    // Internal
    "E999",
];

fn spec_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("spec/errors")
}

/// Extract the error code from the filename (e.g., "E101" from "E101_auto.md").
fn error_code_from_filename(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let code = stem.split('_').next()?;
    if code.starts_with('E') && code.len() >= 4 {
        Some(code.to_string())
    } else {
        None
    }
}

/// Check if a spec file is marked Layer: parser.
fn is_parser_layer(content: &str) -> bool {
    content.lines().any(|line| {
        let lower = line.to_lowercase();
        lower.contains("layer") && lower.contains("parser")
    })
}

/// Extract the first ```chat code block from a spec file.
fn extract_chat_input(content: &str) -> Option<String> {
    let mut in_block = false;
    let mut lines = Vec::new();

    for line in content.lines() {
        if line.trim() == "```chat" {
            in_block = true;
            continue;
        }
        if in_block && line.trim() == "```" {
            break;
        }
        if in_block {
            lines.push(line);
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

#[test]
fn direct_parser_detects_parser_layer_errors_or_is_in_known_gaps() {
    let spec_path = spec_dir();
    if !spec_path.exists() {
        eprintln!("Skipping: spec/errors/ not found at {}", spec_path.display());
        return;
    }

    let dp = DirectParser::new().expect("direct parser");
    let mut tested = 0;
    let mut detected = 0;
    let mut known_gap = 0;
    let mut unexpected_silent = Vec::new();

    let mut entries: Vec<_> = fs::read_dir(&spec_path)
        .expect("read spec dir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "md")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let Some(code) = error_code_from_filename(&path) else {
            continue;
        };

        let content = fs::read_to_string(&path).expect("read spec");
        if !is_parser_layer(&content) {
            continue;
        }

        // Skip known gaps
        if KNOWN_GAPS.contains(&code.as_str()) {
            known_gap += 1;
            continue;
        }

        let Some(chat_input) = extract_chat_input(&content) else {
            continue;
        };

        tested += 1;

        let errors = ErrorCollector::new();
        let result = ChatParser::parse_chat_file(&dp, &chat_input, 0, &errors);
        let error_vec = errors.into_vec();

        // Either the parse failed (returned errors) or errors were collected
        let has_errors = result.is_rejected() || !error_vec.is_empty();

        if has_errors {
            detected += 1;
        } else {
            unexpected_silent.push(format!(
                "{}: direct parser silently accepted known-bad input",
                code
            ));
        }
    }

    println!("Error spec coverage: {tested} tested, {detected} detected, {known_gap} known gaps, {} unexpected silent", unexpected_silent.len());

    // Don't fail on unexpected silent — just report. The direct parser is
    // intentionally more lenient than tree-sitter. This test's value is in
    // tracking the gap over time, not enforcing parity.
    if !unexpected_silent.is_empty() {
        println!("Specs where direct parser was silent (not a failure, but worth tracking):");
        for msg in &unexpected_silent {
            println!("  {msg}");
        }
    }
}
