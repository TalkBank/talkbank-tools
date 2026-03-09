//! Test module for cleaned text overlap in `talkbank-chat`.
//!
//! These tests document expected behavior and regressions.

use super::helpers::{parse_word, snapshot};

// After Phase 2 grammar coarsening, overlap markers (⌈⌉⌊⌋) are INCLUDED in the
// standalone_word token when adjacent to text. They are word-internal characters
// handled by the direct parser (parse_word_impl). The overlap_point grammar rule
// (prec 5, same as standalone_word) only matches when the marker is standalone
// (space-separated); adjacent to text, standalone_word wins by maximal munch.
//
// Direct parser strips overlap markers including their optional index digit and
// optional colon suffix (CHAT overlap notation: ⌈2: is one marker).

/// Verifies inline overlap markers are removed from cleaned text.
#[test]
fn overlap_marker_simple_in_word() {
    // "Yeah⌋⌈2:" → one standalone_word, direct parser strips ⌋ and ⌈2:
    let result = parse_word("Yeah⌋⌈2:");

    if let Ok(word) = &result {
        // cleaned_text strips overlap markers (including index + colon)
        assert_eq!(word.cleaned_text(), "Yeah");
    }

    snapshot(
        "word_parsing_tests__overlap_marker_simple_excluded_from_cleaned_text",
        &result,
    );
}

/// Verifies mid-token overlap markers are removed without dropping lexical characters.
#[test]
fn overlap_marker_midword() {
    // "a⌋2⌈3side" → one standalone_word, direct parser strips ⌋2 and ⌈3
    let result = parse_word("a⌋2⌈3side");

    if let Ok(word) = &result {
        assert_eq!(word.cleaned_text(), "aside");
    }

    snapshot(
        "word_parsing_tests__overlap_marker_midword_excluded",
        &result,
    );
}

/// Verifies indexed overlap markers with suffix punctuation are removed consistently.
#[test]
fn overlap_marker_complex() {
    // "ye⌉2⌊3:s" → one standalone_word; direct parser strips ⌉2 and ⌊3: (colon is part of marker)
    let result = parse_word("ye⌉2⌊3:s");

    if let Ok(word) = &result {
        assert_eq!(word.cleaned_text(), "yes");
    }

    snapshot(
        "word_parsing_tests__overlap_marker_complex_excluded",
        &result,
    );
}

/// Verifies a leading overlap marker is removed from cleaned text.
#[test]
fn overlap_marker_at_beginning() {
    // "⌊hello" → one standalone_word (⌊ is now included in word token)
    // Direct parser parses overlap marker at beginning
    let result = parse_word("⌊hello");

    if let Ok(word) = &result {
        assert_eq!(word.cleaned_text(), "hello");
    }

    snapshot("word_parsing_tests__overlap_marker_at_beginning", &result);
}

/// Verifies numbered overlap markers are removed while preserving surrounding text.
#[test]
fn overlap_marker_numbered() {
    // "test⌋1⌈2ing" → one standalone_word, direct parser strips ⌋1 and ⌈2
    let result = parse_word("test⌋1⌈2ing");

    if let Ok(word) = &result {
        // Direct parser strips overlap markers including their index digits
        assert_eq!(word.cleaned_text(), "testing");
    }

    snapshot(
        "word_parsing_tests__overlap_marker_numbered_excluded",
        &result,
    );
}

/// Verifies all overlap marker variants are stripped during cleaned-text normalization.
#[test]
fn overlap_marker_all_types() {
    // "w⌈or⌉d⌊te⌋st" → one standalone_word with all types of overlap markers
    let result = parse_word("w⌈or⌉d⌊te⌋st");

    if let Ok(word) = &result {
        assert_eq!(word.cleaned_text(), "wordtest");
    }

    snapshot(
        "word_parsing_tests__overlap_marker_all_types_excluded",
        &result,
    );
}
