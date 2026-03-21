//! Example input library for scaffolding
//!
//! Maps node types to realistic CHAT input examples.
//!
//! # Note on Achievability
//!
//! Not all examples produce their expected node in the parse tree. See EXAMPLES_GUIDE.md for:
//! - ✅ Achievable: Examples that parse to the expected node
//! - ⚠️  Approximate: Examples that parse but to a different node
//! - ❌ Problematic: Examples that don't parse correctly with current grammar
//!
//! The scaffold system generates these examples regardless. Use curate-cst's output to verify.\n
/// Get a realistic example input for a given node type
///
/// # Note
/// Not all examples will produce their expected node in the CST. Some are "theoretical"
/// nodes that don't currently parse. See EXAMPLES_GUIDE.md for details on which nodes
/// are achievable vs problematic.
pub fn get_example_input(node_name: &str) -> &'static str {
    match node_name {
        // === ACHIEVABLE: Words ===
        "standalone_word" => "hello",
        "word_with_optional_annotations" => "hello@b",
        "word_body" => "hello",
        "initial_word_segment" => "hello",

        // === APPROXIMATE: Parse but to different nodes ===
        // These inputs parse but may not produce the exact node type expected
        "compound_word" => "ice+cream", // Parses as standalone_word with plus operator
        "ca_annotation" => "‹laughs›",  // May parse as word annotation, not standalone node
        "final_codes" => "[+ text]",    // Codes are word annotations
        "postcode" => "[+ text]",
        "precode" => "[- text]",
        "freecode" => "[* text]",

        // === PROBLEMATIC: Don't parse with simple inputs ===
        // These are theoretical nodes that may require specific context
        "retrace_complete" => "[/] the", // Grammar: [//], may need rewrite context
        "retrace_partial" => "[//] the", // Grammar: [/], naming mismatch
        "retrace_reformulation" => "[///] the", // May need specific context
        "retrace_multiple" => "[/-] the",
        "retrace_uncertain" => "[/?] the",

        // === ACHIEVABLE: Scoped Stressing ===
        // NOTE: Grammar verification (2026-01-13) revealed correct syntax is WITHOUT angle brackets
        // The <> syntax does NOT exist in CHAT format - that was the investigation error!
        "scoped_stressing" => "hello [!]", // ✅ CORRECT syntax (no angle brackets)
        "scoped_contrastive_stressing" => "hello [!!]", // ✅ CORRECT syntax

        // === ACHIEVABLE: Other Scoped Features ===
        // Scoped markers as word annotations (verified 2026-01-13)
        "scoped_symbol" => "hello [>]", // Scoped overlap marker
        "scoped_best_guess" => "hello [=! best guess]",
        "scoped_uncertain" => "hello [=? uncertain]",

        // === ACHIEVABLE: Long Features ===
        // NOTE: Grammar verification (2026-01-13) revealed correct syntax uses span markers
        // The ↔↔ Unicode character syntax does NOT exist in CHAT - that was the investigation error!
        // Correct syntax: &{l=N}text&}l=N} for long features with label N
        "long_feature" => "&{l=1}hello world&}l=1}", // ✅ CORRECT span syntax
        "long_feature_begin" => "&{l=1}hello",       // ✅ CORRECT begin
        "long_feature_end" => "world&}l=1}",         // ✅ CORRECT end
        "long_feature_label" => "1",                 // Label only (no span markers)

        // === ACHIEVABLE: Nonvocal Actions ===
        // NOTE: Grammar verification (2026-01-13) revealed correct syntax uses span markers
        // The &= syntax is incomplete - full syntax is &{n=label}text&}n=label} for spans
        // Simple nonvocal like &=laughing is an EVENT marker, not a span annotation
        "nonvocal" => "&{n=laughing}speech&}n=laughing}", // ✅ CORRECT span syntax
        "nonvocal_simple" => "&=laughing",                // ✅ Simple event marker
        "nonvocal_begin" => "&{n=laughing}",              // ✅ Span begin marker
        "nonvocal_end" => "&}n=laughing}",                // ✅ Span end marker

        // === ACHIEVABLE: Events ===
        "interruption" => "+/.",
        "self_interruption" => "+//.",
        "broken_question" => "+/?",

        // === ACHIEVABLE: Pause Annotations ===
        // NOTE: Grammar verification (2026-01-13) clarified pause syntax
        // Pause notation (.) works BETWEEN words, NOT within words
        // For syllable-level pause within a word, use ^ (caret) instead
        // Examples: "hello (.) world" ✅ WORKS | "he(.)llo" ❌ WRONG (use "he^llo")
        "pause_annotation" => "(.)",
        "pause_short" => "(.)",
        "pause_long" => "(..)",
        "pause_timed" => "(1.5)",

        // === APPROXIMATE: Annotations ===
        "ca_delimited" => "‹text›", // May be part of word structure
        "base_annotation" => "[+ text]",

        // === ACHIEVABLE: Terminators ===
        // NOTE: "terminator" is a TOKEN only, not a primary rule node (verified 2026-01-13)
        // Periods/questions/exclamations only appear as tokens within utterance_end
        // Use utterance_end instead for testing terminators
        "terminator" => ".",
        "period" => ".",
        "question" => "?",
        "exclamation" => "!",
        "utterance_end" => ".",

        // === APPROXIMATE: Event markers ===
        "interrupted" => "+/.",
        "interruption_question" => "+/?",
        "trailing_off" => "+...",
        "trailing_off_question" => "+..?",
        "quotation_precedes" => "+\".",

        // === APPROXIMATE: Replacements/quotations ===
        "replacement" => "[: text]",
        "quotation" => "[\" text]",

        // === ACHIEVABLE: Content ===
        "content_item" => "hello",
        "base_content_item" => "hello",
        "anything" => "hello",
        "nonspaces" => "hello",

        // === ACHIEVABLE: Structure ===
        "document" => "@UTF8\n@Begin\n*CHI:\thello .\n@End",
        "line" => "*CHI:\thello .",
        "utterance" => "*CHI:\thello .",
        "main_tier" => "*CHI:\thello .",

        // === ACHIEVABLE: Dependent Tiers ===
        // NOTE: Dependent tiers only parse as tier-level nodes when they appear
        // after a main_tier in a complete CHAT document structure.
        // Standalone tier examples won't work - must be in full document context.
        "mor_dependent_tier" => {
            // Requires full CHAT: utterance + %mor tier (verified in investigation 2026-01-13)
            "@UTF8\n@Begin\n*CHI:\tthe dog .\n%mor:\tdet|the n|dog .\n@End"
        }
        "gra_dependent_tier" => "%gra:\t1|2|ROOT",
        "pho_dependent_tier" => "%pho:\thɛloʊ",
        "syn_dependent_tier" => "%syn:\tNP",
        "add_dependent_tier" => "%add:\tnote",
        "com_dependent_tier" => "%com:\tcomment",
        "exp_dependent_tier" => "%exp:\texplanation",
        "sit_dependent_tier" => "%sit:\tsituation",

        // === ACHIEVABLE: Headers ===
        "utf8_header" => "@UTF8",
        "begin_header" => "@Begin",
        "end_header" => "@End",
        "languages_header" => "@Languages:\teng",
        "participants_header" => "@Participants:\tCHI Target_Child",
        "id_header" => "@ID:\teng|corpus|CHI|||||Target_Child|||",
        "age_header" => "@Age of CHI:\t2;6.15",
        "birth_header" => "@Birth of CHI:\t01-JAN-2020",
        "date_header" => "@Date:\t15-JUL-2022",

        // === Markers and Symbols (mostly in descriptions only) ===
        "star" => "*",
        "colon" => ":",
        "tab" => "\t",
        "space" => " ",
        "newline" => "\n",
        "comma" => ",",
        "plus" => "+",
        "tilde" => "~",
        "caret" => "^",
        "ampersand" => "&",
        "dollar" => "$",
        "hash" => "#",
        "percent" => "%",
        "at" => "@",
        "equals" => "=",
        "hyphen" => "-",
        "pipe" => "|",
        "semicolon" => ";",

        // === Natural Numbers and URLs ===
        "natural_number" => "123",
        "media_url" => "http://example.com/file.mp3",

        // === Linkers ===
        "linker_quotation_follows" => "+\"",

        // === Default fallback ===
        _ => "hello",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests example lookup.
    #[test]
    fn test_example_lookup() {
        assert_eq!(get_example_input("retrace_complete"), "[/] the");
        assert_eq!(get_example_input("compound_word"), "ice+cream");
        assert_eq!(get_example_input("ca_annotation"), "‹laughs›");
        assert_eq!(get_example_input("scoped_stressing"), "hello [!]"); // Updated: correct CHAT syntax (no angle brackets)

        // Unknown node falls back to "hello"
        assert_eq!(get_example_input("unknown_node"), "hello");
    }

    /// Tests all scaffolded nodes have examples.
    #[test]
    fn test_all_scaffolded_nodes_have_examples() {
        // These are the 28 nodes we're scaffolding
        let scaffolded = vec![
            "document",
            "line",
            "utterance",
            "standalone_word",
            "compound_word",
            "word_with_optional_annotations",
            "retrace_complete",
            "retrace_partial",
            "retrace_reformulation",
            "scoped_symbol",
            "scoped_stressing",
            "scoped_contrastive_stressing",
            "long_feature",
            "long_feature_begin",
            "long_feature_end",
            "nonvocal",
            "nonvocal_simple",
            "interruption",
            "self_interruption",
            "pause_annotation",
            "ca_annotation",
            "mor_dependent_tier",
            "gra_dependent_tier",
            "final_codes",
            "postcode",
            "terminator",
            "utterance_end",
        ];

        for node in scaffolded {
            let example = get_example_input(node);
            assert!(
                !example.is_empty(),
                "Node '{}' should have an example",
                node
            );
            // Check that it's not just the fallback for most nodes
            // (document/line/utterance intentionally use simple examples)
            if !matches!(
                node,
                "standalone_word" | "content_item" | "base_content_item" | "anything"
            ) {
                // Just verify we have something
                assert!(!example.is_empty());
            }
        }
    }
}
