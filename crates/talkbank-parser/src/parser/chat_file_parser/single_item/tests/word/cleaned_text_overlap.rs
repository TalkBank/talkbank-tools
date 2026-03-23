//! Test module for cleaned text with overlap markers in `talkbank-chat`.
//!
//! CHAT principle: contiguous non-whitespace is always one word. Overlap
//! markers (⌈⌉⌊⌋) follow this rule — when adjacent to word text (no space),
//! they are part of the word. When space-separated, they are standalone
//! overlap_point tokens. This applies identically in full-file parsing and
//! in isolated `parse_word()` calls.
//!
//! At the grammar level, overlap_point and standalone_word both have prec(5).
//! Maximal munch makes the longer word_segment match win when markers are
//! adjacent to text. Only space-separated markers match as overlap_point.
//!
//! The cleaned_text() method does NOT strip overlap markers from word text.
//! They are part of the raw word content. Stripping is a downstream concern
//! (e.g., NLP preprocessing) not a parser concern.

use super::helpers::{parse_word, snapshot};

/// Adjacent overlap markers are part of the word text.
#[test]
fn overlap_marker_simple_in_word() {
    // "Yeah⌋⌈2:" → word_segment("Yeah⌋⌈2") + lengthening(":")
    let result = parse_word("Yeah⌋⌈2:");

    if let Ok(word) = &result {
        assert_eq!(word.cleaned_text(), "Yeah⌋⌈2");
    }

    snapshot(
        "word_parsing_tests__overlap_marker_simple_excluded_from_cleaned_text",
        &result,
    );
}

/// Mid-word overlap markers are part of the contiguous word text.
#[test]
fn overlap_marker_midword() {
    let result = parse_word("a⌋2⌈3side");

    if let Ok(word) = &result {
        assert_eq!(word.cleaned_text(), "a⌋2⌈3side");
    }

    snapshot(
        "word_parsing_tests__overlap_marker_midword_excluded",
        &result,
    );
}

/// Overlap markers with colon suffix — colon is lengthening, rest is word.
#[test]
fn overlap_marker_complex() {
    // "ye⌉2⌊3:s" → word_segment("ye⌉2⌊3") + lengthening(":") + word_segment("s")
    let result = parse_word("ye⌉2⌊3:s");

    if let Ok(word) = &result {
        assert_eq!(word.cleaned_text(), "ye⌉2⌊3s");
    }

    snapshot(
        "word_parsing_tests__overlap_marker_complex_excluded",
        &result,
    );
}

/// Leading overlap marker is part of the word (contiguous non-whitespace).
#[test]
fn overlap_marker_at_beginning() {
    let result = parse_word("⌊hello");

    if let Ok(word) = &result {
        assert_eq!(word.cleaned_text(), "⌊hello");
    }

    snapshot("word_parsing_tests__overlap_marker_at_beginning", &result);
}

/// Numbered overlap markers are part of the contiguous word text.
#[test]
fn overlap_marker_numbered() {
    let result = parse_word("test⌋1⌈2ing");

    if let Ok(word) = &result {
        assert_eq!(word.cleaned_text(), "test⌋1⌈2ing");
    }

    snapshot(
        "word_parsing_tests__overlap_marker_numbered_excluded",
        &result,
    );
}

/// All overlap marker variants are part of word text when contiguous.
#[test]
fn overlap_marker_all_types() {
    let result = parse_word("w⌈or⌉d⌊te⌋st");

    if let Ok(word) = &result {
        assert_eq!(word.cleaned_text(), "w⌈or⌉d⌊te⌋st");
    }

    snapshot(
        "word_parsing_tests__overlap_marker_all_types_excluded",
        &result,
    );
}
