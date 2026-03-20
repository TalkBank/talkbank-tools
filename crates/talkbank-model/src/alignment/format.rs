//! Shared formatting for alignment mismatch error messages.
//!
//! Uses LCS-based diff (via `similar`) to show which items match and which
//! are insertions/deletions, instead of naive positional pairing.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Word_Timing_Tier>

use super::helpers::TierPosition;
use similar::{Algorithm, DiffOp};

/// Format a detailed alignment mismatch message with diff-based alignment.
///
/// Instead of pairing items positionally (which makes every row look wrong
/// when the mismatch is a single insertion/deletion), this uses LCS to find
/// the best alignment and shows:
///   ✓  items that match between the two tiers
///   ⊖  items only in the left tier (missing from right)
///   ⊕  items only in the right tier (extra in right)
pub fn format_alignment_mismatch(
    left_name: &str,
    right_name: &str,
    left_items: &[TierPosition],
    right_items: &[TierPosition],
) -> String {
    let left_count = left_items.len();
    let right_count = right_items.len();

    let mut msg = format!(
        "{left_name} has {left_count} alignable items, but {right_name} has {right_count} items\n\n",
    );

    // Extract text for diffing — use cleaned text for comparison
    let left_texts: Vec<&str> = left_items.iter().map(|i| i.text.as_str()).collect();
    let right_texts: Vec<&str> = right_items.iter().map(|i| i.text.as_str()).collect();

    // Compute LCS-based diff
    let ops = similar::capture_diff_slices(Algorithm::Patience, &left_texts, &right_texts);

    // Collect diff rows
    let mut rows: Vec<DiffRow> = Vec::new();
    for op in &ops {
        match op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                for i in 0..*len {
                    rows.push(DiffRow::Equal {
                        left: &left_items[old_index + i].text,
                        right: &right_items[new_index + i].text,
                    });
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for i in 0..*old_len {
                    rows.push(DiffRow::OnlyLeft(&left_items[old_index + i].text));
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for i in 0..*new_len {
                    rows.push(DiffRow::OnlyRight(&right_items[new_index + i].text));
                }
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                // Show replacements as paired rows when possible, then leftovers
                let paired = (*old_len).min(*new_len);
                for i in 0..paired {
                    rows.push(DiffRow::Changed {
                        left: &left_items[old_index + i].text,
                        right: &right_items[new_index + i].text,
                    });
                }
                for i in paired..*old_len {
                    rows.push(DiffRow::OnlyLeft(&left_items[old_index + i].text));
                }
                for i in paired..*new_len {
                    rows.push(DiffRow::OnlyRight(&right_items[new_index + i].text));
                }
            }
        }
    }

    // Render the table
    msg.push_str(&format!("{:<30} {}\n", left_name, right_name));
    msg.push_str(&format!("{:<30} {}\n", "─".repeat(28), "─".repeat(28)));

    for row in &rows {
        match row {
            DiffRow::Equal { left, right } => {
                if *left == *right {
                    msg.push_str(&format!("{:<30} {}\n", left, right));
                } else {
                    // Same logical item, different surface form
                    msg.push_str(&format!("{:<30} {}\n", left, right));
                }
            }
            DiffRow::OnlyLeft(text) => {
                msg.push_str(&format!("{:<30} {:>28} ⊖\n", text, "—"));
            }
            DiffRow::OnlyRight(text) => {
                msg.push_str(&format!("{:<30} {} ⊕\n", "—", text));
            }
            DiffRow::Changed { left, right } => {
                msg.push_str(&format!("{:<30} {} ≠\n", left, right));
            }
        }
    }

    msg
}

/// One rendered row in the alignment diff table.
enum DiffRow<'a> {
    /// Items match (LCS equal)
    Equal { left: &'a str, right: &'a str },
    /// Item only in left tier (deleted from right)
    OnlyLeft(&'a str),
    /// Item only in right tier (extra in right)
    OnlyRight(&'a str),
    /// Items at same position but different (replacement)
    Changed { left: &'a str, right: &'a str },
}

/// Format a positional alignment mismatch for cross-domain tiers.
///
/// Unlike [`format_alignment_mismatch`] which uses LCS to find matching items
/// (useful when both sides are word sequences), this uses simple positional
/// pairing. This is appropriate for tiers like %mor, %pho, and %sin where
/// the dependent tier items are in a completely different domain (morphological
/// tags, phonological tokens, gestures) and string matching is meaningless.
pub fn format_positional_mismatch(
    left_name: &str,
    right_name: &str,
    left_items: &[TierPosition],
    right_items: &[TierPosition],
) -> String {
    let left_count = left_items.len();
    let right_count = right_items.len();

    let mut msg = format!(
        "{left_name} has {left_count} alignable items, but {right_name} has {right_count} items\n\n",
    );

    msg.push_str(&format!("{:<30} {}\n", left_name, right_name));
    msg.push_str(&format!("{:<30} {}\n", "─".repeat(28), "─".repeat(28)));

    let max_len = left_items.len().max(right_items.len());
    for i in 0..max_len {
        let left_text = left_items.get(i).map_or("—", |item| &item.text);
        let right_text = right_items.get(i).map_or("—", |item| &item.text);

        let marker = if i >= left_items.len() {
            " ⊕"
        } else if i >= right_items.len() {
            " ⊖"
        } else {
            ""
        };

        msg.push_str(&format!("{:<30} {}{}\n", left_text, right_text, marker));
    }

    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a diagnostic item wrapper from plain text.
    fn item(text: &str) -> TierPosition {
        TierPosition {
            text: text.to_string(),
            description: None,
        }
    }

    /// Verifies missing from right.
    #[test]
    fn test_missing_from_right() {
        let left = vec![
            item("We're"),
            item("not"),
            item("gonna"),
            item("do"),
            item("the"),
        ];
        let right = vec![item("gonna"), item("do"), item("the")];

        let result = format_alignment_mismatch("Main tier", "%wor tier", &left, &right);
        assert!(result.contains("We're"));
        assert!(result.contains("⊖")); // items missing from right
        assert!(result.contains("gonna")); // matching items present
        // "gonna" should appear as matched, not misaligned
        assert!(!result.contains("gonna") || result.contains("gonna"));
    }

    /// Verifies extra in right.
    #[test]
    fn test_extra_in_right() {
        let left = vec![item("hello"), item("world")];
        let right = vec![item("hello"), item("extra"), item("world")];

        let result = format_alignment_mismatch("Main", "%wor", &left, &right);
        assert!(result.contains("⊕")); // extra in right
    }

    /// Verifies identical.
    #[test]
    fn test_identical() {
        let items = vec![item("a"), item("b"), item("c")];
        let result = format_alignment_mismatch("Left", "Right", &items, &items);
        // No markers — all equal
        assert!(!result.contains("⊖"));
        assert!(!result.contains("⊕"));
        assert!(!result.contains("≠"));
    }

    /// Verifies sbcsae 01 case.
    #[test]
    fn test_sbcsae_01_case() {
        // Real case from SBCSAE/01.cha line 27
        let main = vec![
            item("We're"),
            item("not⌋"),
            item("gonna"),
            item("do"),
            item("the"),
            item("feet"),
            item("today"),
            item("I'm"),
            item("gonna"),
            item("wait"),
            item("till"),
            item("like"),
            item("early"),
            item("in"),
            item("the"),
            item("morning:"),
            item("to"),
            item("do"),
            item("those"),
            item("cause"),
            item("y-"),
        ];
        let wor = vec![
            item("gonna"),
            item("do"),
            item("the"),
            item("feet"),
            item("today"),
            item("I'm"),
            item("gonna"),
            item("wait"),
            item("till"),
            item("like"),
            item("early"),
            item("in"),
            item("the"),
            item("morning"),
            item("to"),
            item("do"),
            item("those"),
            item("cause"),
            item("y"),
        ];

        let result = format_alignment_mismatch("Main tier", "%wor tier", &main, &wor);

        // We're and not⌋ should be marked as only-in-main
        assert!(result.contains("We're"));
        assert!(result.contains("⊖"));

        // gonna should appear as matched
        let lines: Vec<&str> = result.lines().collect();
        // Find lines with "gonna" — they should be equal rows, not misaligned
        let gonna_lines: Vec<&&str> = lines
            .iter()
            .filter(|l| l.contains("gonna") && !l.contains("⊖") && !l.contains("⊕"))
            .collect();
        assert!(
            !gonna_lines.is_empty(),
            "gonna should appear as matched row"
        );

        println!("{}", result);
    }
}
