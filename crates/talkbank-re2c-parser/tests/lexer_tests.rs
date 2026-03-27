//! Lexer tests — verify token sequences for CHAT inputs.
//!
//! The lexer is a standalone public API. Consumers may lex without parsing.
//! These tests verify the re2c lexer produces the correct token sequence
//! for each grammar.js construct.

use talkbank_re2c_parser::lexer::Lexer;
use talkbank_re2c_parser::token::Token;

use talkbank_re2c_parser::lexer::{
    COND_GRA_CONTENT, COND_ID_CONTENT, COND_INITIAL, COND_LANGUAGES_CONTENT, COND_MAIN_CONTENT,
    COND_MEDIA_CONTENT, COND_MOR_CONTENT, COND_PARTICIPANTS_CONTENT, COND_PHO_CONTENT,
    COND_SIN_CONTENT, COND_TIER_CONTENT, COND_TYPES_CONTENT,
};

/// Lex input starting from INITIAL condition.
fn lex(input: &str) -> Vec<Token<'_>> {
    lex_with(input, COND_INITIAL)
}

/// Lex input starting from a specific condition (for isolated tier parsing).
fn lex_with(input: &str, condition: usize) -> Vec<Token<'_>> {
    let mut s = input.to_string();
    s.push('\0');
    let s: &str = Box::leak(s.into_boxed_str());
    Lexer::new(s, condition).map(|(t, _)| t).collect()
}

/// Lex input and return (token, text) pairs for readable assertions.
fn lex_pairs(input: &str) -> Vec<(&'static str, String)> {
    let mut s = input.to_string();
    s.push('\0');
    let s: &str = Box::leak(s.into_boxed_str());
    Lexer::new(s, 0)
        .map(|(t, span)| {
            let name = format!("{:?}", std::mem::discriminant(&t));
            let text = s[span].to_string();
            // Leak the name too for simplicity in tests
            (Box::leak(name.into_boxed_str()) as &str, text)
        })
        .collect()
}

// ── Main tier structure ─────────────────────────────────────────

#[test]
fn lex_simple_main_tier() {
    // grammar.js: main_tier = seq(star, speaker, colon, tab, tier_body)
    let tokens = lex("*CHI:\thello .\n");
    assert!(matches!(tokens[0], Token::Star(_)));
    assert!(matches!(tokens[1], Token::Speaker("CHI")));
    assert!(matches!(tokens[2], Token::TierSep(_))); // :\t
    assert!(
        matches!(
            tokens[3],
            Token::Word {
                body: "hello",
                prefix: None,
                form_marker: None,
                lang_suffix: None,
                ..
            }
        ),
        "got {:?}",
        tokens[3]
    );
    assert!(matches!(tokens[4], Token::Whitespace(_)));
    assert!(matches!(tokens[5], Token::Period(".")));
    assert!(matches!(tokens[6], Token::Newline(_)));
}

#[test]
fn lex_multi_word() {
    let tokens = lex("*MOT:\tdo you want some milk ?\n");
    assert!(matches!(tokens[0], Token::Star(_)));
    assert!(matches!(tokens[1], Token::Speaker("MOT")));
    assert!(matches!(tokens[2], Token::TierSep(_)));
    // do, ws, you, ws, want, ws, some, ws, milk, ws, ?
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::Word { body: "do", .. }))
    );
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::Word { body: "you", .. }))
    );
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::Word { body: "milk", .. }))
    );
    assert!(tokens.iter().any(|t| matches!(t, Token::Question("?"))));
}

// ── Terminators (all from grammar.js) ───────────────────────────

#[test]
fn lex_all_terminators() {
    // Each terminator as the last token before newline
    let cases: Vec<(&str, fn(&Token) -> bool)> = vec![
        ("*X:\tw .\n", |t| matches!(t, Token::Period(_))),
        ("*X:\tw ?\n", |t| matches!(t, Token::Question(_))),
        ("*X:\tw !\n", |t| matches!(t, Token::Exclamation(_))),
        ("*X:\tw +...\n", |t| matches!(t, Token::TrailingOff(_))),
        ("*X:\tw +/.\n", |t| matches!(t, Token::Interruption(_))),
        ("*X:\tw +//.\n", |t| matches!(t, Token::SelfInterruption(_))),
        ("*X:\tw +/?\n", |t| {
            matches!(t, Token::InterruptedQuestion(_))
        }),
        ("*X:\tw +!?\n", |t| matches!(t, Token::BrokenQuestion(_))),
        ("*X:\tw +\"/.\n", |t| matches!(t, Token::QuotedNewLine(_))),
        ("*X:\tw +\".\n", |t| {
            matches!(t, Token::QuotedPeriodSimple(_))
        }),
        ("*X:\tw +//?\n", |t| {
            matches!(t, Token::SelfInterruptedQuestion(_))
        }),
        ("*X:\tw +..?\n", |t| {
            matches!(t, Token::TrailingOffQuestion(_))
        }),
        ("*X:\tw +.\n", |t| matches!(t, Token::BreakForCoding(_))),
        ("*X:\tw \u{2248}\n", |t| matches!(t, Token::CaNoBreak(_))),
        ("*X:\tw \u{224B}\n", |t| {
            matches!(t, Token::CaTechnicalBreak(_))
        }),
    ];

    for (input, check) in &cases {
        let tokens = lex(input);
        let terminator = tokens
            .iter()
            .rev()
            .find(|t| !matches!(t, Token::Newline(_) | Token::Whitespace(_)));
        assert!(
            terminator.is_some_and(check),
            "input {input:?}: expected terminator, got {terminator:?}"
        );
    }
}

// ── Linkers ─────────────────────────────────────────────────────

#[test]
fn lex_linkers() {
    let cases: Vec<(&str, fn(&Token) -> bool)> = vec![
        ("*X:\t+< w .\n", |t| {
            matches!(t, Token::LinkerLazyOverlap(_))
        }),
        ("*X:\t++ w .\n", |t| {
            matches!(t, Token::LinkerQuickUptake(_))
        }),
        ("*X:\t+^ w .\n", |t| {
            matches!(t, Token::LinkerQuickUptakeOverlap(_))
        }),
        ("*X:\t+\" w .\n", |t| {
            matches!(t, Token::LinkerQuotationFollows(_))
        }),
        ("*X:\t+, w .\n", |t| {
            matches!(t, Token::LinkerSelfCompletion(_))
        }),
    ];

    for (input, check) in &cases {
        let tokens = lex(input);
        // Linker should be right after TierSep
        let after_sep = tokens
            .iter()
            .skip_while(|t| !matches!(t, Token::TierSep(_)))
            .nth(1);
        assert!(
            after_sep.is_some_and(check),
            "input {input:?}: expected linker after TierSep, got {after_sep:?}"
        );
    }
}

// ── Atomic annotations (from grammar.js) ────────────────────────

#[test]
fn lex_atomic_annotations() {
    let cases: Vec<(&str, fn(&Token) -> bool)> = vec![
        ("*X:\tw [/] w .\n", |t| {
            matches!(t, Token::RetracePartial(_))
        }),
        ("*X:\tw [//] w .\n", |t| {
            matches!(t, Token::RetraceComplete(_))
        }),
        ("*X:\tw [///] w .\n", |t| {
            matches!(t, Token::RetraceMultiple(_))
        }),
        ("*X:\tw [/-] w .\n", |t| {
            matches!(t, Token::RetraceReformulation(_))
        }),
        ("*X:\tw [/?] w .\n", |t| {
            matches!(t, Token::RetraceUncertain(_))
        }),
        ("*X:\tw [!] .\n", |t| matches!(t, Token::ScopedStressing(_))),
        ("*X:\tw [!!] .\n", |t| {
            matches!(t, Token::ScopedContrastiveStressing(_))
        }),
        ("*X:\tw [!*] .\n", |t| {
            matches!(t, Token::ScopedBestGuess(_))
        }),
        ("*X:\tw [?] .\n", |t| matches!(t, Token::ScopedUncertain(_))),
        ("*X:\tw [e] .\n", |t| matches!(t, Token::ExcludeMarker(_))),
    ];

    for (input, check) in &cases {
        let tokens = lex(input);
        let has = tokens.iter().any(check);
        assert!(has, "input {input:?}: expected annotation, got {tokens:?}");
    }
}

// ── Content annotations ─────────────────────────────────────────

#[test]
fn lex_explanation_annotation() {
    let tokens = lex("*X:\tw [= the cookie] .\n");
    let has = tokens
        .iter()
        .any(|t| matches!(t, Token::ExplanationAnnotation(_)));
    assert!(has, "got {tokens:?}");
}

#[test]
fn lex_para_annotation() {
    let tokens = lex("*X:\tw [=! laughing] .\n");
    let has = tokens.iter().any(|t| matches!(t, Token::ParaAnnotation(_)));
    assert!(has, "got {tokens:?}");
}

#[test]
fn lex_error_marker() {
    let tokens = lex("*X:\tw [*] .\n");
    let has = tokens
        .iter()
        .any(|t| matches!(t, Token::ErrorMarkerAnnotation(_)));
    assert!(has, "got {tokens:?}");
}

#[test]
fn lex_error_marker_with_code() {
    let tokens = lex("*X:\tw [* s:v] .\n");
    let has = tokens
        .iter()
        .any(|t| matches!(t, Token::ErrorMarkerAnnotation(_)));
    assert!(has, "got {tokens:?}");
}

#[test]
fn lex_postcode() {
    let tokens = lex("*X:\tw . [+ bch]\n");
    let has = tokens.iter().any(|t| matches!(t, Token::Postcode(_)));
    assert!(has, "got {tokens:?}");
}

#[test]
fn lex_langcode() {
    let tokens = lex("*X:\tw [- eng] .\n");
    let has = tokens.iter().any(|t| matches!(t, Token::Langcode(_)));
    assert!(has, "got {tokens:?}");
}

// ── Word structure ──────────────────────────────────────────────

#[test]
fn lex_compound_word() {
    // ice+cream is now a single Word token; the compound `+` is inside the body
    let tokens = lex("*X:\tice+cream .\n");
    assert!(
        tokens.iter().any(|t| matches!(
            t,
            Token::Word {
                raw_text: "ice+cream",
                body: "ice+cream",
                ..
            }
        )),
        "expected Word with body 'ice+cream', got {tokens:?}"
    );
}

#[test]
fn lex_lengthening() {
    // no:: is now a single Word token; lengthening `::` is inside the body
    let tokens = lex("*X:\tno:: .\n");
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::Word { body: "no::", .. })),
        "expected Word with body 'no::', got {tokens:?}"
    );
}

#[test]
fn lex_shortening() {
    // (be)cause is now a single Word token; shortening is inside the body
    let tokens = lex("*X:\t(be)cause .\n");
    assert!(
        tokens.iter().any(|t| matches!(
            t,
            Token::Word {
                body: "(be)cause",
                ..
            }
        )),
        "got {tokens:?}"
    );
}

#[test]
fn lex_form_marker() {
    let tokens = lex("*X:\tmama@f .\n");
    assert!(
        tokens.iter().any(|t| matches!(
            t,
            Token::Word {
                body: "mama",
                form_marker: Some("f"),
                ..
            }
        )),
        "got {tokens:?}"
    );
}

#[test]
fn lex_lang_suffix() {
    let tokens = lex("*X:\thao3@s:zho .\n");
    assert!(
        tokens.iter().any(|t| matches!(
            t,
            Token::Word {
                body: "hao3",
                lang_suffix: Some("zho"),
                ..
            }
        )),
        "got {tokens:?}"
    );
}

#[test]
fn lex_lang_suffix_bare() {
    // bare @s → lang_suffix: Some("") (empty string, not None)
    let tokens = lex("*X:\tdog@s .\n");
    assert!(
        tokens.iter().any(|t| matches!(
            t,
            Token::Word {
                body: "dog",
                lang_suffix: Some(""),
                ..
            }
        )),
        "got {tokens:?}"
    );
}

#[test]
fn lex_filler_prefix() {
    let tokens = lex("*X:\t&-um .\n");
    assert!(
        tokens.iter().any(|t| matches!(
            t,
            Token::Word {
                prefix: Some("&-"),
                body: "um",
                ..
            }
        )),
        "got {tokens:?}"
    );
}

#[test]
fn lex_event_marker() {
    let tokens = lex("*X:\t&=laughs .\n");
    assert!(
        tokens.iter().any(|t| matches!(t, Token::EventMarker("&="))),
        "got {tokens:?}"
    );
}

#[test]
fn lex_zero() {
    let tokens = lex("*X:\t0she .\n");
    assert!(
        matches!(
            tokens[3],
            Token::Word {
                prefix: Some("0"),
                body: "she",
                ..
            }
        ),
        "got {:?}",
        tokens[3]
    );
}

#[test]
fn lex_overlap_point() {
    let tokens = lex("*X:\t\u{2308} hello \u{2309} .\n");
    let overlaps: Vec<_> = tokens
        .iter()
        .filter(|t| {
            matches!(
                t,
                Token::OverlapTopBegin(_)
                    | Token::OverlapTopEnd(_)
                    | Token::OverlapBottomBegin(_)
                    | Token::OverlapBottomEnd(_)
            )
        })
        .collect();
    assert_eq!(overlaps.len(), 2, "got {tokens:?}");
    assert!(matches!(overlaps[0], Token::OverlapTopBegin(_)));
    assert!(matches!(overlaps[1], Token::OverlapTopEnd(_)));
}

#[test]
fn lex_stress_marker() {
    // stress marker is now inside the Word body
    let tokens = lex("*X:\t\u{02C8}hello .\n");
    assert!(
        tokens.iter().any(|t| matches!(
            t,
            Token::Word {
                body: "\u{02C8}hello",
                ..
            }
        )),
        "got {tokens:?}"
    );
}

// ── Pauses ──────────────────────────────────────────────────────

#[test]
fn lex_pauses() {
    assert!(
        lex("*X:\t(.) .\n")
            .iter()
            .any(|t| matches!(t, Token::PauseShort(_)))
    );
    assert!(
        lex("*X:\t(..) .\n")
            .iter()
            .any(|t| matches!(t, Token::PauseMedium(_)))
    );
    assert!(
        lex("*X:\t(...) .\n")
            .iter()
            .any(|t| matches!(t, Token::PauseLong(_)))
    );
    assert!(
        lex("*X:\t(1:02.5) .\n")
            .iter()
            .any(|t| matches!(t, Token::PauseTimed(_)))
    );
}

// ── Header lexing ───────────────────────────────────────────────

#[test]
fn lex_header_with_content() {
    // @Languages now enters LANGUAGES_CONTENT with structured tokens
    let tokens = lex("@Languages:\teng\n");
    assert!(
        matches!(tokens[0], Token::HeaderPrefix(s) if s.contains("Languages")),
        "got {:?}",
        tokens[0]
    );
    // Content is now a LanguageCode, not raw HeaderContent
    assert!(
        matches!(tokens[1], Token::LanguageCode("eng")),
        "got {:?}",
        tokens[1]
    );
}

#[test]
fn lex_header_no_content() {
    // @UTF8 now emits a distinct HeaderUtf8 token
    let tokens = lex("@UTF8\n");
    assert!(
        matches!(tokens[0], Token::HeaderUtf8(_)),
        "got {:?}",
        tokens[0]
    );
    assert!(matches!(tokens[1], Token::Newline(_)));
}

// ── Dependent tier prefix ───────────────────────────────────────

#[test]
fn lex_dependent_tier_prefix() {
    // %mor:\t is now a single rich TierPrefix token (includes :\t)
    // and the lexer enters MOR_CONTENT directly
    let tokens = lex("%mor:\tpro|I .\n");
    assert!(matches!(tokens[0], Token::TierPrefix(s) if s.contains("mor")));
    // Next token is MorWord (no separate TierSep — it's baked into TierPrefix)
    assert!(
        matches!(tokens[1], Token::MorWord { .. }),
        "expected MorWord after TierPrefix, got {:?}",
        tokens[1]
    );
}

// ═══════════════════════════════════════════════════════════════
// %mor tier lexing (start in MOR_CONTENT condition)
// grammar.js: mor_word = seq(mor_pos, '|', mor_lemma, repeat(seq('-', mor_feature_value)))
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_mor_simple_word() {
    // "pro|I" → single MorWord token
    let tokens = lex_with("pro|I .\n", COND_MOR_CONTENT);
    assert!(
        matches!(
            tokens[0],
            Token::MorWord {
                pos: "pro",
                lemma_features: "I"
            }
        ),
        "expected MorWord(\"pro|I\"), got {:?}",
        tokens[0]
    );
}

#[test]
fn lex_mor_word_with_features() {
    // "verb|want-Fin-Ind-Pres" → single MorWord token
    let tokens = lex_with("verb|want-Fin-Ind-Pres .\n", COND_MOR_CONTENT);
    assert!(
        matches!(
            tokens[0],
            Token::MorWord {
                pos: "verb",
                lemma_features: "want-Fin-Ind-Pres"
            }
        ),
        "got {:?}",
        tokens[0]
    );
}

#[test]
fn lex_mor_multiple_items() {
    // "pro|I v|want n|cookie-PL ." → MorWord, ws, MorWord, ws, MorWord, ws, Period
    let tokens = lex_with("pro|I v|want n|cookie-PL .\n", COND_MOR_CONTENT);
    let mor_words: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t, Token::MorWord { .. }))
        .collect();
    assert_eq!(mor_words.len(), 3, "expected 3 mor words, got {tokens:?}");
    assert!(matches!(
        mor_words[0],
        Token::MorWord {
            pos: "pro",
            lemma_features: "I"
        }
    ));
    assert!(matches!(
        mor_words[1],
        Token::MorWord {
            pos: "v",
            lemma_features: "want"
        }
    ));
    assert!(matches!(
        mor_words[2],
        Token::MorWord {
            pos: "n",
            lemma_features: "cookie-PL"
        }
    ));
}

#[test]
fn lex_mor_with_clitic() {
    // "pron|it~aux|be-Fin-Ind-Pres-S3" → MorWord, Tilde, MorWord
    let tokens = lex_with("pron|it~aux|be-Fin-Ind-Pres-S3 .\n", COND_MOR_CONTENT);
    assert!(
        matches!(
            tokens[0],
            Token::MorWord {
                pos: "pron",
                lemma_features: "it"
            }
        ),
        "got {:?}",
        tokens[0]
    );
    assert!(
        matches!(tokens[1], Token::MorTilde("~")),
        "got {:?}",
        tokens[1]
    );
    assert!(
        matches!(tokens[2], Token::MorWord { pos: "aux", lemma_features } if lemma_features.starts_with("be")),
        "got {:?}",
        tokens[2]
    );
}

#[test]
fn lex_mor_pos_with_subcategory() {
    // "pro:sub|I" → single MorWord (colon in POS is allowed)
    let tokens = lex_with("pro:sub|I .\n", COND_MOR_CONTENT);
    assert!(
        matches!(
            tokens[0],
            Token::MorWord {
                pos: "pro:sub",
                lemma_features: "I"
            }
        ),
        "got {:?}",
        tokens[0]
    );
}

#[test]
fn lex_mor_real_corpus_line() {
    // From corpus/reference/tiers/mor-gra.cha
    let tokens = lex_with(
        "pron|it~aux|be-Fin-Ind-Pres-S3 pron|I-Prs-Nom-S1 verb|want-Fin-Ind-Pres noun|cookie-Plur .\n",
        COND_MOR_CONTENT,
    );
    let mor_words: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t, Token::MorWord { .. }))
        .collect();
    // 5 mor words: pron|it, aux|be-..., pron|I-..., verb|want-..., noun|cookie-...
    assert_eq!(mor_words.len(), 5, "got {mor_words:?}");
    assert!(matches!(tokens.last().unwrap(), Token::Newline(_)));
}

#[test]
fn lex_mor_with_terminator() {
    let tokens = lex_with("pro|I .\n", COND_MOR_CONTENT);
    assert!(tokens.iter().any(|t| matches!(t, Token::Period(_))));
}

// ═══════════════════════════════════════════════════════════════
// %gra tier lexing (start in GRA_CONTENT condition)
// grammar.js: gra_relation = seq(gra_index, '|', gra_head, '|', gra_relation_name)
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_gra_single_relation() {
    // "1|2|SUBJ" → single GraRelation token
    let tokens = lex_with("1|2|SUBJ\n", COND_GRA_CONTENT);
    assert!(
        matches!(
            tokens[0],
            Token::GraRelation {
                index: "1",
                head: "2",
                relation: "SUBJ"
            }
        ),
        "got {:?}",
        tokens[0]
    );
}

#[test]
fn lex_gra_multiple_relations() {
    // "1|4|NSUBJ 2|1|AUX 3|4|NSUBJ 4|0|ROOT 5|4|OBJ 6|4|PUNCT"
    let tokens = lex_with(
        "1|4|NSUBJ 2|1|AUX 3|4|NSUBJ 4|0|ROOT 5|4|OBJ 6|4|PUNCT\n",
        COND_GRA_CONTENT,
    );
    let relations: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t, Token::GraRelation { .. }))
        .collect();
    assert_eq!(relations.len(), 6, "got {tokens:?}");
    assert!(matches!(
        relations[0],
        Token::GraRelation {
            index: "1",
            head: "4",
            relation: "NSUBJ"
        }
    ));
    assert!(matches!(
        relations[3],
        Token::GraRelation {
            index: "4",
            head: "0",
            relation: "ROOT"
        }
    ));
    assert!(matches!(
        relations[5],
        Token::GraRelation {
            index: "6",
            head: "4",
            relation: "PUNCT"
        }
    ));
}

#[test]
fn lex_gra_relation_with_hyphen() {
    // Relation names can have hyphens: "AUX-PASS"
    let tokens = lex_with("1|2|AUX-PASS\n", COND_GRA_CONTENT);
    assert!(
        matches!(
            tokens[0],
            Token::GraRelation {
                index: "1",
                head: "2",
                relation: "AUX-PASS"
            }
        ),
        "got {:?}",
        tokens[0]
    );
}

// ═══════════════════════════════════════════════════════════════
// Generic tier content lexing (start in TIER_CONTENT condition)
// grammar.js: text_with_bullets = repeat1(choice(text_segment, inline_bullet, continuation))
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_tier_content_text() {
    let tokens = lex_with("CHI is standing on the couch\n", COND_TIER_CONTENT);
    assert!(matches!(
        tokens[0],
        Token::TextSegment("CHI is standing on the couch")
    ));
}

#[test]
fn lex_tier_content_with_bullet() {
    // Text with inline bullet
    let tokens = lex_with("hello \u{0015}100_200\u{0015} world\n", COND_TIER_CONTENT);
    assert!(tokens.iter().any(|t| matches!(t, Token::TextSegment(_))));
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::MediaBullet { .. }))
    );
}

// ═══════════════════════════════════════════════════════════════
// %pho tier lexing (start in PHO_CONTENT)
// grammar.js: pho_word = /[IPA chars]+/, plus joins compounds
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_pho_simple_word() {
    let tokens = lex_with("hello\n", COND_PHO_CONTENT);
    assert!(
        matches!(tokens[0], Token::PhoWord("hello")),
        "got {:?}",
        tokens[0]
    );
}

#[test]
fn lex_pho_ipa() {
    // From corpus: %pho: ˈɑmɪ
    let tokens = lex_with("\u{02C8}\u{0251}m\u{026A}\n", COND_PHO_CONTENT);
    assert!(
        matches!(tokens[0], Token::PhoWord(_)),
        "got {:?}",
        tokens[0]
    );
}

#[test]
fn lex_pho_compound() {
    // From corpus: %pho: wɑ+kɪŋ
    let tokens = lex_with("w\u{0251}+k\u{026A}\u{014B}\n", COND_PHO_CONTENT);
    let pho_words: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t, Token::PhoWord(_)))
        .collect();
    assert_eq!(pho_words.len(), 2, "got {tokens:?}");
    assert!(tokens.iter().any(|t| matches!(t, Token::PhoPlus(_))));
}

#[test]
fn lex_pho_multiple_words() {
    let tokens = lex_with("a b c\n", COND_PHO_CONTENT);
    let pho_words: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t, Token::PhoWord(_)))
        .collect();
    assert_eq!(pho_words.len(), 3, "got {tokens:?}");
}

#[test]
fn lex_pho_with_group() {
    // ‹word1 word2›
    let tokens = lex_with("\u{2039}a b\u{203A}\n", COND_PHO_CONTENT);
    assert!(tokens.iter().any(|t| matches!(t, Token::PhoGroupBegin(_))));
    assert!(tokens.iter().any(|t| matches!(t, Token::PhoGroupEnd(_))));
}

// ═══════════════════════════════════════════════════════════════
// %sin tier lexing (start in SIN_CONTENT)
// grammar.js: sin_word = choice(zero, /[a-zA-Z0-9:_-]+/)
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_sin_simple_word() {
    let tokens = lex_with("g:toy:dpoint\n", COND_SIN_CONTENT);
    assert!(
        matches!(tokens[0], Token::SinWord("g:toy:dpoint")),
        "got {:?}",
        tokens[0]
    );
}

#[test]
fn lex_sin_multiple_words() {
    let tokens = lex_with("point give hold\n", COND_SIN_CONTENT);
    let words: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t, Token::SinWord(_)))
        .collect();
    assert_eq!(words.len(), 3, "got {tokens:?}");
}

#[test]
fn lex_sin_with_group() {
    // 〔word1 word2〕
    let tokens = lex_with("\u{3014}a b\u{3015}\n", COND_SIN_CONTENT);
    assert!(tokens.iter().any(|t| matches!(t, Token::SinGroupBegin(_))));
    assert!(tokens.iter().any(|t| matches!(t, Token::SinGroupEnd(_))));
}

// ═══════════════════════════════════════════════════════════════
// %wor tier lexing (start in WOR_CONTENT)
// grammar.js: wor_tier_body — words + inline_bullets + separators
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_wor_words() {
    let tokens = lex_with("hello world\n", COND_MAIN_CONTENT);
    let words: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t, Token::Word { .. }))
        .collect();
    assert_eq!(words.len(), 2, "got {tokens:?}");
}

#[test]
fn lex_wor_with_bullet() {
    let tokens = lex_with("hello \u{0015}100_200\u{0015} world\n", COND_MAIN_CONTENT);
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::Word { body: "hello", .. })),
        "expected Word 'hello', got {tokens:?}"
    );
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::MediaBullet { .. }))
    );
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::Word { body: "world", .. })),
        "expected Word 'world', got {tokens:?}"
    );
}

// ═══════════════════════════════════════════════════════════════
// Full dependent tier lines (start in INITIAL — tier dispatch)
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_full_mor_line() {
    // From INITIAL: %mor:\t dispatches directly to MOR_CONTENT
    let tokens = lex("%mor:\tpro|I v|want .\n");
    assert!(matches!(tokens[0], Token::TierPrefix(s) if s.contains("mor")));
    // After the rich TierPrefix("%mor:\t"), we're in MOR_CONTENT
    // so the next tokens should be MorWord, not generic TextSegment
    assert!(
        tokens.iter().any(|t| matches!(t, Token::MorWord { .. })),
        "expected MorWord tokens after %mor prefix, got {tokens:?}"
    );
}

#[test]
fn lex_full_gra_line() {
    // From INITIAL: %gra:\t dispatches directly to GRA_CONTENT
    let tokens = lex("%gra:\t1|2|SUBJ 2|0|ROOT\n");
    assert!(matches!(tokens[0], Token::TierPrefix(s) if s.contains("gra")));
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::GraRelation { .. })),
        "expected GraRelation tokens after %gra prefix, got {tokens:?}"
    );
}

#[test]
fn lex_full_pho_line() {
    let tokens = lex("%pho:\ta b c\n");
    assert!(matches!(tokens[0], Token::TierPrefix(s) if s.contains("pho")));
    assert!(
        tokens.iter().any(|t| matches!(t, Token::PhoWord(_))),
        "expected PhoWord tokens after %pho prefix, got {tokens:?}"
    );
}

#[test]
fn lex_full_sin_line() {
    let tokens = lex("%sin:\tpoint give\n");
    assert!(matches!(tokens[0], Token::TierPrefix(s) if s.contains("sin")));
    assert!(
        tokens.iter().any(|t| matches!(t, Token::SinWord(_))),
        "expected SinWord tokens after %sin prefix, got {tokens:?}"
    );
}

#[test]
fn lex_full_com_line() {
    // %com is a generic text tier
    let tokens = lex("%com:\tCHI is standing\n");
    assert!(matches!(tokens[0], Token::TierPrefix(s) if s.contains("com")));
    assert!(
        tokens.iter().any(|t| matches!(t, Token::TextSegment(_))),
        "expected TextSegment after %com prefix, got {tokens:?}"
    );
}

#[test]
fn lex_full_wor_line() {
    // %wor dispatches to MAIN_CONTENT (same word rules)
    let tokens = lex("%wor:\thello world\n");
    assert!(matches!(tokens[0], Token::TierPrefix(s) if s.contains("wor")));
    assert!(
        tokens.iter().any(|t| matches!(t, Token::Word { .. })),
        "expected Word token after %wor prefix, got {tokens:?}"
    );
}

// ═══════════════════════════════════════════════════════════════
// No-content headers (distinct tokens)
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_header_utf8() {
    let tokens = lex("@UTF8\n");
    assert!(
        matches!(tokens[0], Token::HeaderUtf8("@UTF8")),
        "got {:?}",
        tokens[0]
    );
}

#[test]
fn lex_header_begin() {
    let tokens = lex("@Begin\n");
    assert!(
        matches!(tokens[0], Token::HeaderBegin("@Begin")),
        "got {:?}",
        tokens[0]
    );
}

#[test]
fn lex_header_end() {
    let tokens = lex("@End\n");
    assert!(
        matches!(tokens[0], Token::HeaderEnd("@End")),
        "got {:?}",
        tokens[0]
    );
}

#[test]
fn lex_header_new_episode() {
    let tokens = lex("@New Episode\n");
    assert!(
        matches!(tokens[0], Token::HeaderNewEpisode(_)),
        "got {:?}",
        tokens[0]
    );
}

// ═══════════════════════════════════════════════════════════════
// @ID — ultra-rich token with tagged field boundaries
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_id_header_full() {
    let tokens = lex("@ID:\teng|corpus|CHI|3;0|female|typical||Child|||\n");
    assert!(
        matches!(tokens[0], Token::HeaderPrefix(s) if s.contains("ID")),
        "got {:?}",
        tokens[0]
    );
    assert!(
        matches!(
            tokens[1],
            Token::IdFields {
                language: "eng",
                ..
            }
        ),
        "expected IdFields with language=eng, got {:?}",
        tokens[1]
    );
    assert!(matches!(tokens[2], Token::Newline(_)));
}

#[test]
fn lex_id_header_minimal() {
    let tokens = lex("@ID:\teng|corpus|MOT|||||Mother|||\n");
    assert!(
        matches!(
            tokens[1],
            Token::IdFields {
                language: "eng",
                speaker: "MOT",
                ..
            }
        ),
        "got {:?}",
        tokens[1]
    );
}

#[test]
fn lex_id_content_isolated() {
    let tokens = lex_with(
        "eng|corpus|CHI|3;0|female|typical||Child|||\n",
        COND_ID_CONTENT,
    );
    match &tokens[0] {
        Token::IdFields {
            language,
            corpus,
            speaker,
            age,
            sex,
            group,
            ses,
            role,
            education,
            custom,
        } => {
            assert_eq!(*language, "eng");
            assert_eq!(*corpus, "corpus");
            assert_eq!(*speaker, "CHI");
            assert_eq!(*age, "3;0");
            assert_eq!(*sex, "female");
            assert_eq!(*group, "typical");
            assert_eq!(*ses, "");
            assert_eq!(*role, "Child");
            assert_eq!(*education, "");
            assert_eq!(*custom, "");
        }
        _ => panic!("expected IdFields, got {:?}", tokens[0]),
    }
}

// ═══════════════════════════════════════════════════════════════
// @Types — rich token with 3 tagged fields
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_types_header() {
    let tokens = lex("@Types:\tlongitudinal, naturalistic, TD\n");
    assert!(matches!(tokens[0], Token::HeaderPrefix(s) if s.contains("Types")));
    assert!(
        matches!(
            tokens[1],
            Token::TypesFields {
                design: "longitudinal",
                ..
            }
        ),
        "got {:?}",
        tokens[1]
    );
}

#[test]
fn lex_types_content_isolated() {
    let tokens = lex_with("cross, toyplay, TD\n", COND_TYPES_CONTENT);
    match &tokens[0] {
        Token::TypesFields {
            design,
            activity,
            group,
        } => {
            assert_eq!(*design, "cross");
            assert_eq!(*activity, "toyplay");
            assert_eq!(*group, "TD");
        }
        _ => panic!("expected TypesFields, got {:?}", tokens[0]),
    }
}

// ═══════════════════════════════════════════════════════════════
// Specific header prefixes (verify dispatch)
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_specific_header_prefixes() {
    // Headers that still use opaque HeaderContent
    let raw_text_cases = [
        "@Date:\t28-JUL-2001\n",
        "@Location:\tPittsburgh, PA\n",
        "@Options:\tCA\n",
        "@PID:\t11312/c-00016447-1\n",
        "@Recording Quality:\t4\n",
    ];
    for input in &raw_text_cases {
        let tokens = lex(input);
        assert!(
            matches!(tokens[0], Token::HeaderPrefix(_)),
            "input {input:?}: expected HeaderPrefix, got {:?}",
            tokens[0]
        );
        assert!(
            matches!(tokens[1], Token::HeaderContent(_)),
            "input {input:?}: expected HeaderContent, got {:?}",
            tokens[1]
        );
    }

    // Headers with structured content (not HeaderContent)
    // @Languages → LanguageCode tokens
    let tokens = lex("@Languages:\teng\n");
    assert!(matches!(tokens[0], Token::HeaderPrefix(_)));
    assert!(matches!(tokens[1], Token::LanguageCode(_)));

    // @Participants → ParticipantWord tokens
    let tokens = lex("@Participants:\tCHI Child\n");
    assert!(matches!(tokens[0], Token::HeaderPrefix(_)));
    assert!(matches!(tokens[1], Token::ParticipantWord(_)));

    // @Comment → TextSegment (bullet-aware, via TIER_CONTENT)
    let tokens = lex("@Comment:\tSome text\n");
    assert!(matches!(tokens[0], Token::HeaderPrefix(_)));
    assert!(matches!(tokens[1], Token::TextSegment(_)));

    // @Media → MediaWord tokens
    let tokens = lex("@Media:\tfile, audio\n");
    assert!(matches!(tokens[0], Token::HeaderPrefix(_)));
    assert!(matches!(tokens[1], Token::MediaWord(_)));
}

#[test]
fn lex_optional_content_headers() {
    // @Bg with content
    let tokens = lex("@Bg:\tsome gem label\n");
    assert!(matches!(tokens[0], Token::HeaderPrefix(s) if s.contains("Bg")));
    assert!(matches!(tokens[1], Token::HeaderContent(_)));

    // @Bg without content (just newline)
    let tokens = lex("@Bg\n");
    assert!(matches!(tokens[0], Token::HeaderPrefix(s) if s.contains("Bg")));
    assert!(matches!(tokens[1], Token::Newline(_)));
}

// ═══════════════════════════════════════════════════════════════
// @Languages — structured comma-separated language codes
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_languages_single() {
    let tokens = lex("@Languages:\teng\n");
    assert!(matches!(tokens[0], Token::HeaderPrefix(_)));
    assert!(
        matches!(tokens[1], Token::LanguageCode("eng")),
        "got {:?}",
        tokens[1]
    );
}

#[test]
fn lex_languages_multiple() {
    let tokens = lex("@Languages:\teng, fra, zho\n");
    let codes: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t, Token::LanguageCode(_)))
        .collect();
    assert_eq!(codes.len(), 3, "got {tokens:?}");
}

#[test]
fn lex_languages_isolated() {
    let tokens = lex_with("eng, fra\n", COND_LANGUAGES_CONTENT);
    let codes: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t, Token::LanguageCode(_)))
        .collect();
    assert_eq!(codes.len(), 2);
    assert!(tokens.iter().any(|t| matches!(t, Token::Comma(_))));
}

// ═══════════════════════════════════════════════════════════════
// @Participants — structured comma-separated entries
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_participants() {
    let tokens = lex("@Participants:\tCHI Child, MOT Mother\n");
    assert!(matches!(tokens[0], Token::HeaderPrefix(_)));
    let words: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t, Token::ParticipantWord(_)))
        .collect();
    // CHI, Child, MOT, Mother = 4 words
    assert_eq!(words.len(), 4, "got {tokens:?}");
    assert!(tokens.iter().any(|t| matches!(t, Token::Comma(_))));
}

#[test]
fn lex_participants_isolated() {
    let tokens = lex_with(
        "CHI Child, MOT Mother, FAT Father\n",
        COND_PARTICIPANTS_CONTENT,
    );
    let words: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t, Token::ParticipantWord(_)))
        .collect();
    assert_eq!(words.len(), 6); // CHI Child MOT Mother FAT Father
    let commas: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t, Token::Comma(_)))
        .collect();
    assert_eq!(commas.len(), 2);
}

// ═══════════════════════════════════════════════════════════════
// @Media — structured filename, type[, status]
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_media_simple() {
    let tokens = lex("@Media:\tfile, audio\n");
    assert!(matches!(tokens[0], Token::HeaderPrefix(_)));
    let media_words: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t, Token::MediaWord(_)))
        .collect();
    assert_eq!(media_words.len(), 2, "file + audio, got {tokens:?}");
}

#[test]
fn lex_media_with_status() {
    let tokens = lex("@Media:\trecording, video, missing\n");
    let media_words: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t, Token::MediaWord(_)))
        .collect();
    assert_eq!(media_words.len(), 3, "recording + video + missing");
}

#[test]
fn lex_media_quoted_filename() {
    let tokens = lex("@Media:\t\"http://example.com/file.mp4\", audio\n");
    assert!(
        tokens.iter().any(|t| matches!(t, Token::MediaFilename(_))),
        "expected quoted filename, got {tokens:?}"
    );
}

// ═══════════════════════════════════════════════════════════════
// @Comment — text_with_bullets_and_pics (NOT raw text!)
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_comment_with_bullet() {
    let tokens = lex("@Comment:\tsome text \u{0015}100_200\u{0015} more text\n");
    assert!(tokens.iter().any(|t| matches!(t, Token::TextSegment(_))));
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::MediaBullet { .. })),
        "expected bullet in @Comment, got {tokens:?}"
    );
}

#[test]
fn lex_comment_with_inline_pic() {
    let tokens = lex("@Comment:\ttext \u{0015}%pic:\"image.jpg\"\u{0015} more\n");
    assert!(
        tokens.iter().any(|t| matches!(t, Token::InlinePic(_))),
        "expected inline pic in @Comment, got {tokens:?}"
    );
}

// ═══════════════════════════════════════════════════════════════
// Tier content with bullets
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_tier_content_with_inline_pic() {
    let tokens = lex_with(
        "text \u{0015}%pic:\"img.jpg\"\u{0015} more\n",
        COND_TIER_CONTENT,
    );
    assert!(tokens.iter().any(|t| matches!(t, Token::InlinePic(_))));
}

// ═══════════════════════════════════════════════════════════════
// @Birth of SPK — speaker embedded in prefix
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_birth_of_header() {
    let tokens = lex("@Birth of CHI:\t28-JUL-2001\n");
    assert!(
        matches!(tokens[0], Token::HeaderBirthOf("CHI")),
        "expected HeaderBirthOf(\"CHI\"), got {:?}",
        tokens[0]
    );
    assert!(matches!(tokens[1], Token::HeaderContent("28-JUL-2001")));
}

// ═══════════════════════════════════════════════════════════════
// Error recovery — lexer never fails, always returns error tokens
// ═══════════════════════════════════════════════════════════════

#[test]
fn lex_error_recovery_main_content() {
    // \x07 (BEL) is not a valid main content char → ErrorInMainContent, then continue
    let tokens = lex("*X:\thello \x07 world .\n");
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::ErrorInMainContent(_))),
        "expected error token, got {tokens:?}"
    );
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::Word { body: "world", .. })),
        "expected Word 'world' after error recovery, got {tokens:?}"
    );
    assert!(tokens.iter().any(|t| matches!(t, Token::Period(_))));
}

#[test]
fn lex_error_recovery_mor_content() {
    let tokens = lex_with("pro|I \x07 v|want .\n", COND_MOR_CONTENT);
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::ErrorInMorContent(_)))
    );
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::MorWord { pos: "v", .. }))
    );
}

#[test]
fn lex_error_recovery_gra_content() {
    let tokens = lex_with("1|2|SUBJ \x07 3|0|ROOT\n", COND_GRA_CONTENT);
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::ErrorInGraContent(_)))
    );
    assert!(tokens.iter().any(|t| matches!(
        t,
        Token::GraRelation {
            relation: "ROOT",
            ..
        }
    )));
}

#[test]
fn lex_error_recovery_invalid_line() {
    let tokens = lex("GARBAGE\n*X:\thello .\n");
    assert!(
        matches!(tokens[0], Token::ErrorLine("GARBAGE")),
        "got {:?}",
        tokens[0]
    );
    assert!(tokens.iter().any(|t| matches!(t, Token::Star(_))));
}

#[test]
fn lex_error_unclosed_paren() {
    let tokens = lex("*X:\thello (unclosed .\n");
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::ErrorUnclosedParen(_)))
    );
    assert!(tokens.iter().any(|t| matches!(t, Token::Period(_))));
}

#[test]
fn lex_never_panics_on_arbitrary_input() {
    let inputs = [
        "\x00",
        "\x01\x02\x03\x04",
        "@@@@",
        "****",
        "%%%%",
        "[[[[]]]",
        "(((())))",
        "++++++",
        "\t\t\t\t",
        "\r\n\r\n\r\n",
        "*:\t.\n",
        "%:\t.\n",
        "@:\t.\n",
        "*CHI:\t\x07\x08\x01\x02 .\n",
        "%mor:\t\x07|\x08 .\n",
    ];
    for input in &inputs {
        let mut s = input.to_string();
        s.push('\0');
        let s: &str = Box::leak(s.into_boxed_str());
        let tokens: Vec<_> = Lexer::new(s, 0).collect();
        let _ = tokens; // just verify no panic
    }
}

// ── Corpus smoke test ───────────────────────────────────────────

#[test]
fn lex_corpus_main_tiers_no_panic() {
    let base =
        std::path::Path::new(&std::env::var("HOME").unwrap_or_else(|_| "/Users/chen".to_string()))
            .join("talkbank/talkbank-tools/corpus/reference");

    if !base.exists() {
        return;
    }

    let mut total = 0;
    for dir in ["core", "content", "annotation", "tiers", "ca", "languages"] {
        let dir_path = base.join(dir);
        if !dir_path.exists() {
            continue;
        }
        for entry in std::fs::read_dir(&dir_path).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "cha") {
                let content = std::fs::read_to_string(&path).unwrap();
                for line in content.lines() {
                    if line.starts_with('*') {
                        total += 1;
                        let input = format!("{line}\n\0");
                        let tokens: Vec<_> = Lexer::new(&input, 0).collect();
                        assert!(!tokens.is_empty(), "empty lex for: {line}");
                    }
                }
            }
        }
    }
    eprintln!("Lexed {total} main tier lines without panic");
}

// ═══════════════════════════════════════════════════════════════
// InlinePic in TIER_CONTENT
// ═══════════════════════════════════════════════════════════════

#[test]
fn tier_content_inline_pic() {
    // %com content: text followed by \u0015%pic:"filename"\u0015
    let input = "pic002 \u{0015}%pic:\"a18/image002.jpg\"\u{0015}\n";
    let tokens = lex_with(input, COND_TIER_CONTENT);
    eprintln!("tokens: {tokens:?}");
    assert!(
        tokens.iter().any(|t| matches!(t, Token::InlinePic(_))),
        "expected InlinePic token, got: {tokens:?}"
    );
}

#[test]
fn tier_content_inline_pic_standalone() {
    // Just the pic marker
    let input = "\u{0015}%pic:\"photo.jpg\"\u{0015}\n";
    let tokens = lex_with(input, COND_TIER_CONTENT);
    eprintln!("tokens: {tokens:?}");
    assert!(
        tokens.iter().any(|t| matches!(t, Token::InlinePic(_))),
        "expected InlinePic token, got: {tokens:?}"
    );
}

// ═══════════════════════════════════════════════════════════════
// NonvocalSimple vs NonvocalBegin
// ═══════════════════════════════════════════════════════════════

#[test]
fn nonvocal_simple_vs_begin() {
    // &{n=BANG} is nonvocal_simple — token carries just the label "BANG"
    let tokens = lex_with("&{n=BANG} what\n", COND_MAIN_CONTENT);
    assert!(
        matches!(tokens[0], Token::NonvocalSimple("BANG")),
        "expected NonvocalSimple(\"BANG\"), got: {:?}",
        tokens[0]
    );

    // &{n=THUMP without } is nonvocal_begin — carries just "THUMP"
    let tokens2 = lex_with("&{n=THUMP word &}n=THUMP\n", COND_MAIN_CONTENT);
    assert!(
        matches!(tokens2[0], Token::NonvocalBegin("THUMP")),
        "expected NonvocalBegin(\"THUMP\"), got: {:?}",
        tokens2[0]
    );
    assert!(
        tokens2
            .iter()
            .any(|t| matches!(t, Token::NonvocalEnd("THUMP"))),
        "expected NonvocalEnd(\"THUMP\"), got: {tokens2:?}"
    );
}

#[test]
fn long_feature_tags() {
    // &{l=X carries just "X", &}l=PAR carries just "PAR"
    let tokens = lex_with("&{l=X deal &}l=X\n", COND_MAIN_CONTENT);
    assert!(
        matches!(tokens[0], Token::LongFeatureBegin("X")),
        "expected LongFeatureBegin(\"X\"), got: {:?}",
        tokens[0]
    );
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::LongFeatureEnd("X"))),
        "expected LongFeatureEnd(\"X\"), got: {tokens:?}"
    );
}
