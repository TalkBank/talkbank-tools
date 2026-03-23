//! Legacy main-tier and utterance fragment parity tests against the reference corpus.
//!
//! These tests ensure `parse_main_tier()` and `parse_utterance()` match the same
//! parser's whole-file AST on real utterances drawn from the sacred reference corpus.
//!
//! This is still useful for auditing synthetic fragment behavior, but it should
//! not be treated as the long-term oracle for direct-parser fragment semantics.

use std::fs;
use std::path::PathBuf;

use talkbank_model::ChatOptionFlag;
use talkbank_model::ErrorCollector;
use talkbank_model::model::{Line, SemanticEq};
use talkbank_model::{ChatParser, FragmentSemanticContext, ParseOutcome};
use talkbank_parser_tests::test_error::TestError;

use super::parser_impl::parser_suite;

fn reference_root() -> PathBuf {
    let mut full = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    full.push("../../corpus/reference");
    full
}

fn reference_file(path: &str) -> PathBuf {
    let mut full = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    full.push(path);
    full
}

fn collect_reference_files() -> Result<Vec<PathBuf>, TestError> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(reference_root()) {
        let entry = entry.map_err(|err| TestError::Failure(format!("walkdir failure: {err}")))?;
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "cha") {
            files.push(entry.into_path());
        }
    }
    files.sort();
    Ok(files)
}

fn relative_display(path: &std::path::Path) -> String {
    let root = reference_root();
    path.strip_prefix(&root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn source_uses_file_context_options(source: &str) -> bool {
    source.lines().any(|line| line.starts_with("@Options:\t"))
}

#[test]
fn reference_main_tiers_roundtrip_for_every_parser() -> Result<(), TestError> {
    let files = collect_reference_files()?;

    for parser in parser_suite()? {
        for path in &files {
            let source = fs::read_to_string(path).map_err(|err| {
                TestError::Failure(format!("failed to read {}: {err}", path.display()))
            })?;

            // `parse_main_tier()` is a context-free fragment API. Files with
            // `@Options:` can change whole-file main-tier semantics (for example
            // CA omission normalization), so they are audited separately rather
            // than being treated as parity failures here.
            if source_uses_file_context_options(&source) {
                continue;
            }

            let file_errors = ErrorCollector::new();
            let parsed_file = match ChatParser::parse_chat_file(&parser, &source, 0, &file_errors) {
                ParseOutcome::Parsed(file) => file,
                ParseOutcome::Rejected => {
                    return Err(TestError::Failure(format!(
                        "[{}] rejected whole-file parse for {}",
                        parser.parser_name(),
                        path.display()
                    )));
                }
            };

            if !file_errors.is_empty() {
                return Err(TestError::Failure(format!(
                    "[{}] whole-file parse errors for {}: {:?}",
                    parser.parser_name(),
                    path.display(),
                    file_errors.to_vec()
                )));
            }

            for utterance in parsed_file.utterances() {
                let main_line = utterance.main.to_chat();
                let main_errors = ErrorCollector::new();
                let reparsed = match ChatParser::parse_main_tier(&parser, &main_line, 0, &main_errors) {
                    ParseOutcome::Parsed(main) => main,
                    ParseOutcome::Rejected => {
                        return Err(TestError::Failure(format!(
                            "[{}] parse_main_tier rejected `{}` from {}",
                            parser.parser_name(),
                            main_line,
                            relative_display(path)
                        )));
                    }
                };

                if !main_errors.is_empty() {
                    return Err(TestError::Failure(format!(
                        "[{}] parse_main_tier errors for `{}` from {}: {:?}",
                        parser.parser_name(),
                        main_line,
                        relative_display(path),
                        main_errors.to_vec()
                    )));
                }

                if !utterance.main.semantic_eq(&reparsed) {
                    return Err(TestError::Failure(format!(
                        "[{}] parse_main_tier semantic mismatch for `{}` from {}",
                        parser.parser_name(),
                        main_line,
                        relative_display(path)
                    )));
                }
            }
        }
    }

    Ok(())
}

#[test]
fn reference_utterances_roundtrip_for_every_parser() -> Result<(), TestError> {
    let files = [
        "../../corpus/reference/core/basic-conversation.cha",
        "../../corpus/reference/core/headers-comments.cha",
        "../../corpus/reference/content/media-bullets.cha",
        "../../corpus/reference/tiers/multi-tier-utterance.cha",
        "../../corpus/reference/tiers/mor-gra.cha",
    ];

    for parser in parser_suite()? {
        for relative_path in files {
            let path = reference_file(relative_path);
            let source = fs::read_to_string(&path).map_err(|err| {
                TestError::Failure(format!("failed to read {}: {err}", path.display()))
            })?;

            let file_errors = ErrorCollector::new();
            let parsed_file = match ChatParser::parse_chat_file(&parser, &source, 0, &file_errors) {
                ParseOutcome::Parsed(file) => file,
                ParseOutcome::Rejected => {
                    return Err(TestError::Failure(format!(
                        "[{}] rejected whole-file parse for {}",
                        parser.parser_name(),
                        path.display()
                    )));
                }
            };

            if !file_errors.is_empty() {
                return Err(TestError::Failure(format!(
                    "[{}] whole-file parse errors for {}: {:?}",
                    parser.parser_name(),
                    path.display(),
                    file_errors.to_vec()
                )));
            }

            for line in &parsed_file.lines {
                let Line::Utterance(utterance) = line else {
                    continue;
                };

                let utterance_text = utterance.to_chat();
                let utterance_errors = ErrorCollector::new();
                let reparsed = match ChatParser::parse_utterance(&parser, &utterance_text, 0, &utterance_errors) {
                    ParseOutcome::Parsed(utterance) => utterance,
                    ParseOutcome::Rejected => {
                        return Err(TestError::Failure(format!(
                            "[{}] parse_utterance rejected `{}` from {}",
                            parser.parser_name(),
                            utterance_text,
                            path.display()
                        )));
                    }
                };

                if !utterance_errors.is_empty() {
                    return Err(TestError::Failure(format!(
                        "[{}] parse_utterance errors for `{}` from {}: {:?}",
                        parser.parser_name(),
                        utterance_text,
                        path.display(),
                        utterance_errors.to_vec()
                    )));
                }

                if !utterance.as_ref().semantic_eq(&reparsed) {
                    return Err(TestError::Failure(format!(
                        "[{}] parse_utterance semantic mismatch for `{}` from {}",
                        parser.parser_name(),
                        utterance_text,
                        path.display()
                    )));
                }
            }
        }
    }

    Ok(())
}

#[test]
fn main_tier_ca_omission_requires_file_context_for_every_parser() -> Result<(), TestError> {
    let input = "*CHI:\t(word) .";

    for parser in parser_suite()? {
        let errors = ErrorCollector::new();
        let parsed = ChatParser::parse_main_tier(&parser, input, 0, &errors);

        if !parsed.is_rejected() {
            return Err(TestError::Failure(format!(
                "[{}] parse_main_tier unexpectedly accepted CA-omission fragment without file context",
                parser.parser_name()
            )));
        }

        if errors.is_empty() {
            return Err(TestError::Failure(format!(
                "[{}] parse_main_tier rejected CA-omission fragment without reporting an error",
                parser.parser_name()
            )));
        }
    }

    Ok(())
}

#[test]
fn main_tier_ca_omission_parses_with_ca_context_for_every_parser() -> Result<(), TestError> {
    let input = "*CHI:\t(word) .";
    let context = FragmentSemanticContext::new().with_option_flag(ChatOptionFlag::Ca);

    for parser in parser_suite()? {
        let errors = ErrorCollector::new();
        let parsed = match parser.parse_main_tier_with_context(input, 0, &context, &errors) {
            ParseOutcome::Parsed(main) => main,
            ParseOutcome::Rejected => {
                return Err(TestError::Failure(format!(
                    "[{}] parse_main_tier_with_context rejected CA-omission fragment with CA context",
                    parser.parser_name()
                )));
            }
        };

        if !errors.is_empty() {
            return Err(TestError::Failure(format!(
                "[{}] parse_main_tier_with_context reported errors with CA context: {:?}",
                parser.parser_name(),
                errors.to_vec()
            )));
        }

        let serialized = parsed.to_chat();
        if serialized != input {
            return Err(TestError::Failure(format!(
                "[{}] parse_main_tier_with_context roundtrip mismatch: expected `{}`, got `{}`",
                parser.parser_name(),
                input,
                serialized
            )));
        }
    }

    Ok(())
}
