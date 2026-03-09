//! Shared segment-splitting utilities for tier-level error recovery.
//!
//! The direct parser is batch-oriented and uses chumsky combinators that
//! are fail-fast at the tier level: one bad item rejects the entire tier.
//! This module provides utilities to split tier content into whitespace-
//! delimited segments so each can be parsed independently, enabling
//! item-level recovery where good items are kept and bad ones are skipped.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
/// A whitespace-delimited segment of tier content with its byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TierSegment<'a> {
    /// The text of this segment.
    pub text: &'a str,
    /// Byte offset of this segment within the parent tier content string.
    pub offset: usize,
}

/// Split tier content into whitespace-delimited segments.
///
/// CHAT whitespace for tier content is: space, `\n\t` (LF continuation),
/// or `\r\n\t` (CRLF continuation). Splits on any of these, returning
/// non-empty segments with their byte offsets.
pub(crate) fn split_tier_segments(input: &str) -> Vec<TierSegment<'_>> {
    let mut segments = Vec::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Skip whitespace
        i = skip_chat_whitespace(bytes, i);
        if i >= len {
            break;
        }

        // Scan non-whitespace segment
        let start = i;
        while i < len && !is_chat_ws_start(bytes, i) {
            i += 1;
        }

        if i > start {
            segments.push(TierSegment {
                text: &input[start..i],
                offset: start,
            });
        }
    }

    segments
}

/// Split %pho tier content into segments, treating `‹...›` regions as atomic.
///
/// U+2039 (‹) and U+203A (›) delimit phonological groups that contain
/// internal spaces. These regions are kept as single segments.
pub(crate) fn split_pho_tier_segments(input: &str) -> Vec<TierSegment<'_>> {
    let mut segments = Vec::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    // U+2039 = E2 80 B9, U+203A = E2 80 BA in UTF-8
    const OPEN: [u8; 3] = [0xE2, 0x80, 0xB9];
    const CLOSE: [u8; 3] = [0xE2, 0x80, 0xBA];

    while i < len {
        // Skip whitespace
        i = skip_chat_whitespace(bytes, i);
        if i >= len {
            break;
        }

        let start = i;

        // Check for group opener ‹
        if i + 3 <= len && bytes[i..i + 3] == OPEN {
            // Scan until closing ›
            i += 3;
            loop {
                if i + 3 <= len && bytes[i..i + 3] == CLOSE {
                    i += 3; // consume the closer
                    break;
                }
                if i >= len {
                    break; // unclosed group — segment ends at EOF
                }
                i += 1;
            }
        } else {
            // Regular segment: scan until whitespace or group opener
            while i < len && !is_chat_ws_start(bytes, i) {
                if i + 3 <= len && bytes[i..i + 3] == OPEN {
                    break;
                }
                i += 1;
            }
        }

        if i > start {
            segments.push(TierSegment {
                text: &input[start..i],
                offset: start,
            });
        }
    }

    segments
}

/// Check whether the given segment matches one of the 13 CHAT terminator forms.
///
/// This is a simple string comparison (not a parser) for use in recovery loops
/// to identify the trailing terminator segment.
pub(crate) fn is_mor_terminator(text: &str) -> bool {
    matches!(
        text,
        "." | "?"
            | "!"
            | "+..."
            | "+//."
            | "+/."
            | "+//?"
            | "+/?"
            | "+!?"
            | "+\"/."
            | "+\"."
            | "+..?"
            | "+."
    )
}

/// Extract the speaker code from a raw main tier line via cheap byte scan.
///
/// Looks for the pattern `*CODE:\t` and returns the CODE portion.
/// Returns `None` if the line doesn't match the expected format.
pub(crate) fn extract_speaker_code(content: &str) -> Option<&str> {
    let bytes = content.as_bytes();

    // Must start with *
    if bytes.first().copied() != Some(b'*') {
        return None;
    }

    // Find :\t
    let sep_pos = bytes.windows(2).position(|w| w == b":\t")?;
    if sep_pos <= 1 {
        return None; // empty speaker code
    }

    // Speaker code is between * and :
    let code = &content[1..sep_pos];

    // Validate: speaker codes are typically 3 uppercase ASCII chars
    if code.is_empty() || !code.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }

    Some(code)
}

/// Skip CHAT whitespace starting at position `i`.
/// Returns the new position after all whitespace.
fn skip_chat_whitespace(bytes: &[u8], mut i: usize) -> usize {
    let len = bytes.len();
    while i < len {
        if bytes[i] == b' ' {
            i += 1;
        } else if i + 2 <= len && bytes[i] == b'\n' && bytes[i + 1] == b'\t' {
            i += 2; // \n\t continuation
        } else if i + 3 <= len
            && bytes[i] == b'\r'
            && bytes[i + 1] == b'\n'
            && bytes[i + 2] == b'\t'
        {
            i += 3; // \r\n\t continuation
        } else {
            break;
        }
    }
    i
}

/// Check if position `i` is the start of CHAT whitespace (space or continuation).
fn is_chat_ws_start(bytes: &[u8], i: usize) -> bool {
    let len = bytes.len();
    if bytes[i] == b' ' {
        return true;
    }
    if bytes[i] == b'\n' && i + 1 < len && bytes[i + 1] == b'\t' {
        return true;
    }
    if bytes[i] == b'\r' && i + 2 < len && bytes[i + 1] == b'\n' && bytes[i + 2] == b'\t' {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Splits simple segments.
    #[test]
    fn split_simple_segments() {
        let segs = split_tier_segments("pron|I verb|see det|the .");
        assert_eq!(segs.len(), 4);
        assert_eq!(segs[0].text, "pron|I");
        assert_eq!(segs[0].offset, 0);
        assert_eq!(segs[1].text, "verb|see");
        assert_eq!(segs[1].offset, 7);
        assert_eq!(segs[2].text, "det|the");
        assert_eq!(segs[2].offset, 16);
        assert_eq!(segs[3].text, ".");
        assert_eq!(segs[3].offset, 24);
    }

    /// Splits with continuation lines.
    #[test]
    fn split_with_continuation_lines() {
        let segs = split_tier_segments("pron|I\n\tverb|see .");
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].text, "pron|I");
        assert_eq!(segs[1].text, "verb|see");
        assert_eq!(segs[2].text, ".");
    }

    /// Splits empty input.
    #[test]
    fn split_empty_input() {
        let segs = split_tier_segments("");
        assert!(segs.is_empty());
    }

    /// Splits whitespace only.
    #[test]
    fn split_whitespace_only() {
        let segs = split_tier_segments("   ");
        assert!(segs.is_empty());
    }

    /// Tests pho segments group atomic.
    #[test]
    fn pho_segments_group_atomic() {
        let segs = split_pho_tier_segments("hɛˈloʊ ‹gʊd baɪ› .");
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].text, "hɛˈloʊ");
        assert_eq!(segs[1].text, "‹gʊd baɪ›");
        assert_eq!(segs[2].text, ".");
    }

    /// Tests pho segments no groups.
    #[test]
    fn pho_segments_no_groups() {
        let segs = split_pho_tier_segments("hɛˈloʊ ðɛr");
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].text, "hɛˈloʊ");
        assert_eq!(segs[1].text, "ðɛr");
    }

    /// Tests terminator detection.
    #[test]
    fn terminator_detection() {
        assert!(is_mor_terminator("."));
        assert!(is_mor_terminator("?"));
        assert!(is_mor_terminator("!"));
        assert!(is_mor_terminator("+..."));
        assert!(is_mor_terminator("+//."));
        assert!(is_mor_terminator("+/."));
        assert!(is_mor_terminator("+//?"));
        assert!(is_mor_terminator("+/?"));
        assert!(is_mor_terminator("+!?"));
        assert!(is_mor_terminator("+\"/."));
        assert!(is_mor_terminator("+\"."));
        assert!(is_mor_terminator("+..?"));
        assert!(is_mor_terminator("+."));
        // Non-terminators
        assert!(!is_mor_terminator("pron|I"));
        assert!(!is_mor_terminator("verb|go"));
        assert!(!is_mor_terminator(""));
    }

    /// Extracts speaker code valid.
    #[test]
    fn extract_speaker_code_valid() {
        assert_eq!(extract_speaker_code("*CHI:\thello ."), Some("CHI"));
        assert_eq!(extract_speaker_code("*MOT:\thi ."), Some("MOT"));
        assert_eq!(extract_speaker_code("*INV:\ttest ."), Some("INV"));
    }

    #[test]
    fn extract_speaker_code_invalid() {
        assert_eq!(extract_speaker_code("%mor:\tpron|I"), None);
        assert_eq!(extract_speaker_code("@Begin"), None);
        assert_eq!(extract_speaker_code("hello"), None);
        assert_eq!(extract_speaker_code("*:\t"), None); // empty code
    }

    // =========================================================================
    // Offset tracking (mutant-targeted)
    // =========================================================================

    #[test]
    fn split_segments_tracks_offsets_correctly() {
        let segs = split_tier_segments("ab cd ef");
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].text, "ab");
        assert_eq!(segs[0].offset, 0);
        assert_eq!(segs[1].text, "cd");
        assert_eq!(segs[1].offset, 3);
        assert_eq!(segs[2].text, "ef");
        assert_eq!(segs[2].offset, 6);
    }

    #[test]
    fn split_segments_with_multiple_spaces() {
        let segs = split_tier_segments("a   b");
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].text, "a");
        assert_eq!(segs[0].offset, 0);
        assert_eq!(segs[1].text, "b");
        assert_eq!(segs[1].offset, 4);
    }

    #[test]
    fn split_segments_crlf_continuation() {
        let segs = split_tier_segments("hello\r\n\tworld .");
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].text, "hello");
        assert_eq!(segs[1].text, "world");
        assert_eq!(segs[2].text, ".");
    }

    #[test]
    fn split_segments_continuation_offset() {
        let segs = split_tier_segments("first\n\tsecond");
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].offset, 0);
        assert_eq!(segs[1].offset, 7); // "first\n\t" = 7 bytes
        assert_eq!(segs[1].text, "second");
    }

    #[test]
    fn split_segments_single_word() {
        let segs = split_tier_segments("only");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, "only");
        assert_eq!(segs[0].offset, 0);
    }

    // =========================================================================
    // Pho group segment tests (mutant-targeted)
    // =========================================================================

    #[test]
    fn pho_segments_unclosed_group_extends_to_eof() {
        let segs = split_pho_tier_segments("‹unclosed group");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, "‹unclosed group");
    }

    #[test]
    fn pho_segments_multiple_groups() {
        let segs = split_pho_tier_segments("‹a b› ‹c d›");
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].text, "‹a b›");
        assert_eq!(segs[1].text, "‹c d›");
    }

    #[test]
    fn pho_segments_mixed_groups_and_words() {
        let segs = split_pho_tier_segments("a ‹b c› d");
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].text, "a");
        assert_eq!(segs[1].text, "‹b c›");
        assert_eq!(segs[2].text, "d");
    }

    #[test]
    fn pho_segments_empty_input() {
        let segs = split_pho_tier_segments("");
        assert!(segs.is_empty());
    }

    #[test]
    fn pho_segments_offsets_with_groups() {
        let segs = split_pho_tier_segments("ab ‹cd ef›");
        assert_eq!(segs[0].offset, 0);
        assert_eq!(segs[0].text, "ab");
        assert_eq!(segs[1].offset, 3);
        // ‹ is 3 bytes UTF-8
        assert!(segs[1].text.starts_with('‹'));
    }

    // =========================================================================
    // Whitespace helpers (mutant-targeted)
    // =========================================================================

    #[test]
    fn skip_chat_whitespace_skips_spaces() {
        assert_eq!(skip_chat_whitespace(b"   abc", 0), 3);
    }

    #[test]
    fn skip_chat_whitespace_skips_continuation() {
        assert_eq!(skip_chat_whitespace(b"\n\tabc", 0), 2);
    }

    #[test]
    fn skip_chat_whitespace_skips_crlf_continuation() {
        assert_eq!(skip_chat_whitespace(b"\r\n\tabc", 0), 3);
    }

    #[test]
    fn skip_chat_whitespace_stops_at_text() {
        assert_eq!(skip_chat_whitespace(b"abc", 0), 0);
    }

    #[test]
    fn skip_chat_whitespace_from_middle() {
        assert_eq!(skip_chat_whitespace(b"abc  def", 3), 5);
    }

    #[test]
    fn is_chat_ws_start_detects_space() {
        assert!(is_chat_ws_start(b"a b", 1));
    }

    #[test]
    fn is_chat_ws_start_detects_lf_tab() {
        assert!(is_chat_ws_start(b"a\n\tb", 1));
    }

    #[test]
    fn is_chat_ws_start_detects_crlf_tab() {
        assert!(is_chat_ws_start(b"a\r\n\tb", 1));
    }

    #[test]
    fn is_chat_ws_start_rejects_lone_lf() {
        assert!(!is_chat_ws_start(b"a\nb", 1));
    }

    #[test]
    fn is_chat_ws_start_rejects_text() {
        assert!(!is_chat_ws_start(b"abc", 1));
    }

    // =========================================================================
    // Speaker code extraction edge cases
    // =========================================================================

    #[test]
    fn extract_speaker_empty_input() {
        assert_eq!(extract_speaker_code(""), None);
    }

    #[test]
    fn extract_speaker_non_alphanumeric_rejected() {
        assert_eq!(extract_speaker_code("*CH!:\thello ."), None);
    }

    #[test]
    fn extract_speaker_numeric_allowed() {
        assert_eq!(extract_speaker_code("*SP1:\thello ."), Some("SP1"));
    }

    // =========================================================================
    // Terminator non-matches
    // =========================================================================

    #[test]
    fn terminator_rejects_partial_matches() {
        assert!(!is_mor_terminator("+"));
        assert!(!is_mor_terminator(".."));
        assert!(!is_mor_terminator("+.."));
        assert!(!is_mor_terminator("//"));
    }
}
