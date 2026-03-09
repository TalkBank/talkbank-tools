//! Diagnostic harness for checking incremental utterance reparsing behavior.
//!

use std::env;
use std::fs;
use std::io;
use std::time::Instant;

use talkbank_model::model::{Line, Utterance};
use talkbank_model::{ErrorCollector, ErrorSink};
use talkbank_parser::{ParserInitError, TreeSitterParser};
use thiserror::Error;
use tree_sitter::{Node, Range as TsRange, Tree};

/// Return `true` when the CST node kind corresponds to any CHAT header variant.
fn is_header_kind(kind: &str) -> bool {
    use talkbank_parser::node_types::{
        ACTIVITIES_HEADER, BCK_HEADER, BG_HEADER, BIRTH_OF_HEADER, BIRTHPLACE_OF_HEADER,
        BLANK_HEADER, COLOR_WORDS_HEADER, COMMENT_HEADER, DATE_HEADER, EG_HEADER, FONT_HEADER,
        G_HEADER, HEADER, ID_HEADER, L1_OF_HEADER, LANGUAGES_HEADER, LOCATION_HEADER, MEDIA_HEADER,
        NEW_EPISODE_HEADER, NUMBER_HEADER, OPTIONS_HEADER, PAGE_HEADER, PARTICIPANTS_HEADER,
        PID_HEADER, PRE_BEGIN_HEADER, RECORDING_QUALITY_HEADER, ROOM_LAYOUT_HEADER,
        SITUATION_HEADER, T_HEADER, TAPE_LOCATION_HEADER, TIME_DURATION_HEADER, TIME_START_HEADER,
        TRANSCRIBER_HEADER, TRANSCRIPTION_HEADER, TYPES_HEADER, VIDEOS_HEADER, WARNING_HEADER,
        WINDOW_HEADER,
    };

    matches!(
        kind,
        HEADER
            | PRE_BEGIN_HEADER
            | ACTIVITIES_HEADER
            | BCK_HEADER
            | BG_HEADER
            | BIRTH_OF_HEADER
            | BIRTHPLACE_OF_HEADER
            | BLANK_HEADER
            | COLOR_WORDS_HEADER
            | COMMENT_HEADER
            | DATE_HEADER
            | EG_HEADER
            | FONT_HEADER
            | G_HEADER
            | ID_HEADER
            | L1_OF_HEADER
            | LANGUAGES_HEADER
            | LOCATION_HEADER
            | MEDIA_HEADER
            | NEW_EPISODE_HEADER
            | NUMBER_HEADER
            | OPTIONS_HEADER
            | PAGE_HEADER
            | PARTICIPANTS_HEADER
            | PID_HEADER
            | RECORDING_QUALITY_HEADER
            | ROOM_LAYOUT_HEADER
            | SITUATION_HEADER
            | T_HEADER
            | TAPE_LOCATION_HEADER
            | TIME_DURATION_HEADER
            | TIME_START_HEADER
            | TRANSCRIBER_HEADER
            | TRANSCRIPTION_HEADER
            | TYPES_HEADER
            | VIDEOS_HEADER
            | WARNING_HEADER
            | WINDOW_HEADER
    )
}

/// Check whether two byte ranges intersect.
fn ranges_overlap(start_a: usize, end_a: usize, start_b: usize, end_b: usize) -> bool {
    start_a < end_b && start_b < end_a
}

/// Collect top-level utterance CST nodes from a parsed CHAT tree.
fn collect_utterance_nodes<'a>(tree: &'a Tree) -> Vec<Node<'a>> {
    let mut utterances = Vec::new();
    let root = tree.root_node();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.is_missing() || child.is_error() {
            continue;
        }
        if child.kind() == talkbank_parser::node_types::LINE {
            let mut line_cursor = child.walk();
            for line_child in child.children(&mut line_cursor) {
                if line_child.is_missing() || line_child.is_error() {
                    continue;
                }
                if line_child.kind() == talkbank_parser::node_types::UTTERANCE {
                    utterances.push(line_child);
                }
            }
        } else if is_header_kind(child.kind()) {
            continue;
        }
    }
    utterances
}

/// Map changed byte ranges to utterance-node indices that overlap those edits.
fn affected_utterance_indices<'a>(
    utterance_nodes: &[Node<'a>],
    changed_ranges: &[TsRange],
) -> Vec<usize> {
    let mut indices = Vec::new();
    for (idx, node) in utterance_nodes.iter().enumerate() {
        let start = node.start_byte();
        let end = node.end_byte();
        if changed_ranges
            .iter()
            .any(|range| ranges_overlap(start, end, range.start_byte, range.end_byte))
        {
            indices.push(idx);
        }
    }
    indices
}

/// Compute a conservative changed range directly from old/new text bytes.
fn compute_text_changed_range(old_text: &str, new_text: &str) -> Option<TsRange> {
    if old_text == new_text {
        return None;
    }

    let old_bytes = old_text.as_bytes();
    let new_bytes = new_text.as_bytes();
    let mut start = 0;
    let min_len = old_bytes.len().min(new_bytes.len());
    while start < min_len && old_bytes[start] == new_bytes[start] {
        start += 1;
    }

    let mut old_end = old_bytes.len();
    let mut new_end = new_bytes.len();
    while old_end > start && new_end > start && old_bytes[old_end - 1] == new_bytes[new_end - 1] {
        old_end -= 1;
        new_end -= 1;
    }

    Some(TsRange {
        start_byte: start,
        end_byte: new_end,
        start_point: byte_to_point(new_text, start),
        end_point: byte_to_point(new_text, new_end),
    })
}

/// Convert a UTF-8 byte offset into a tree-sitter `(row, column)` point.
fn byte_to_point(text: &str, byte: usize) -> tree_sitter::Point {
    let mut row = 0;
    let mut column = 0;
    let mut count = 0;
    for ch in text.chars() {
        if count >= byte {
            break;
        }
        if ch == '\n' {
            row += 1;
            column = 0;
        } else {
            column += ch.len_utf16();
        }
        count += ch.len_utf8();
    }

    tree_sitter::Point { row, column }
}

/// Collect chat-file line indices corresponding to utterance entries.
fn collect_utterance_line_indices(chat_file: &talkbank_model::model::ChatFile) -> Vec<usize> {
    let mut indices = Vec::new();
    for (idx, line) in chat_file.lines.iter().enumerate() {
        if matches!(line, Line::Utterance(_)) {
            indices.push(idx);
        }
    }
    indices
}

/// Pick a deterministic in-utterance alphabetic edit position for the benchmark edit.
fn find_edit_position_from_tree(text: &str, tree: &Tree) -> Option<usize> {
    let utterances = collect_utterance_nodes(tree);
    let first = utterances.first()?;
    let start = first.start_byte();
    let end = first.end_byte().min(text.len());
    let slice = &text[start..end];
    for (idx, ch) in slice.char_indices() {
        if ch.is_ascii_alphabetic() {
            return Some(start + idx);
        }
    }
    None
}

/// Apply a one-character synthetic edit used to compare full vs incremental rebuilds.
fn edit_text(text: &str, tree: &Tree) -> (String, bool) {
    let mut out = text.to_string();
    if let Some(pos) = find_edit_position_from_tree(text, tree) {
        let current = out.as_bytes()[pos] as char;
        let replacement = if current == 'x' { 'y' } else { 'x' };
        out.replace_range(pos..pos + 1, &replacement.to_string());
        return (out, true);
    }
    (out, false)
}

/// Parse an entire CHAT file using the full parser path.
fn parse_full(
    parser: &TreeSitterParser,
    text: &str,
) -> talkbank_model::ParseResult<talkbank_model::model::ChatFile> {
    parser.parse_chat_file(text)
}

/// Error variants surfaced by the incremental-check harness.
#[derive(Debug, Error)]
enum IncrementalError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Parser init error: {0}")]
    ParserInit(#[from] ParserInitError),
    #[error("Parse error: {0}")]
    Parse(#[from] talkbank_model::ParseErrors),
    #[error("{0}")]
    Message(String),
}

/// Reparse a single utterance node from CST into model form.
fn parse_utterance_from_cst(
    parser: &TreeSitterParser,
    node: Node,
    source: &str,
    errors: &impl ErrorSink,
) -> Option<Utterance> {
    parser
        .parse_utterance_cst(node, source, errors)
        .into_option()
}

/// Run a full-vs-incremental parse comparison for one input file.
fn main() -> Result<(), IncrementalError> {
    let mut args = env::args();
    let _ = args.next();
    let Some(path) = args.next() else {
        eprintln!("Usage: incremental_lsp_check <file.cha>");
        std::process::exit(2);
    };

    let text = fs::read_to_string(&path)?;
    let parser = TreeSitterParser::new()?;

    let full_start = Instant::now();
    let mut old_chat_file = parse_full(&parser, &text)?;
    let full_time = full_start.elapsed();

    let old_tree = parser.parse_tree_incremental(&text, None)?;

    let (edited, edited_ok) = edit_text(&text, &old_tree);
    let edited_tree = parser.parse_tree_incremental(&edited, Some(&old_tree))?;

    let mut changed_ranges: Vec<TsRange> = old_tree.changed_ranges(&edited_tree).collect();
    if changed_ranges.is_empty()
        && let Some(range) = compute_text_changed_range(&text, &edited)
    {
        changed_ranges.push(range);
    }
    let utterance_nodes = collect_utterance_nodes(&edited_tree);
    let affected = affected_utterance_indices(&utterance_nodes, &changed_ranges);
    let line_indices = collect_utterance_line_indices(&old_chat_file);

    let incremental_start = Instant::now();
    let parse_errors = ErrorCollector::new();
    for idx in &affected {
        let Some(line_idx) = line_indices.get(*idx) else {
            return Err(IncrementalError::Message(
                "Invalid utterance index mapping".to_string(),
            ));
        };
        let Some(utterance) =
            parse_utterance_from_cst(&parser, utterance_nodes[*idx], &edited, &parse_errors)
        else {
            return Err(IncrementalError::Message(
                "Utterance parse failed".to_string(),
            ));
        };
        old_chat_file.lines[*line_idx] = Line::utterance(utterance);
    }
    let incremental_time = incremental_start.elapsed();

    let edited_full_start = Instant::now();
    let edited_full = parse_full(&parser, &edited)?;
    let edited_full_time = edited_full_start.elapsed();

    let parse_errors = parse_errors.into_vec();
    if !parse_errors.is_empty() {
        eprintln!("Incremental parse errors: {}", parse_errors.len());
    }

    let matches = old_chat_file == edited_full;

    println!("File: {}", path);
    println!("Full parse time: {:?}", full_time);
    println!("Incremental rebuild time: {:?}", incremental_time);
    println!("Edited full parse time: {:?}", edited_full_time);
    println!("Edit applied: {}", edited_ok);
    println!("Changed ranges: {}", changed_ranges.len());
    println!("Affected utterances: {}", affected.len());
    println!("Match full parse: {}", matches);
    Ok(())
}
