//! Study what TreeSitterParser produces for specific inputs.
//! This is research, not a test suite — run with --nocapture to see output.
//!
//! cargo test -p talkbank-re2c-parser --test model_study -- --nocapture --ignored

use talkbank_model::SemanticEq;
use talkbank_parser::TreeSitterParser;

fn ts() -> TreeSitterParser {
    TreeSitterParser::new().expect("grammar loads")
}

#[test]
#[ignore]
fn study_simple_file() {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Target_Child, MOT Mother\n@ID:\teng|corpus|CHI|3;0||||Child|||\n*CHI:\thello world .\n%mor:\tn|hello n|world .\n%gra:\t1|2|SUBJ 2|0|ROOT .\n@End\n";
    let file = ts().parse_chat_file(input).unwrap();
    eprintln!("{}", serde_json::to_string_pretty(&file).unwrap());
}

#[test]
#[ignore]
fn study_main_tier_simple() {
    let mt = ts().parse_main_tier("*CHI:\thello world .\n").unwrap();
    eprintln!("{}", serde_json::to_string_pretty(&mt).unwrap());
}

#[test]
#[ignore]
fn study_main_tier_compound() {
    let mt = ts().parse_main_tier("*CHI:\tI want ice+cream .\n").unwrap();
    eprintln!("{}", serde_json::to_string_pretty(&mt).unwrap());
}

#[test]
#[ignore]
fn study_main_tier_annotations() {
    let mt = ts().parse_main_tier("*CHI:\tthe the [/] dog .\n").unwrap();
    eprintln!("{}", serde_json::to_string_pretty(&mt).unwrap());
}

#[test]
#[ignore]
fn study_word_simple() {
    let w = ts().parse_word("hello").unwrap();
    eprintln!("{}", serde_json::to_string_pretty(&w).unwrap());
}

// ═══════════════════════════════════════════════════════════════
// Word equivalence tests (not ignored — these run in CI)
// ═══════════════════════════════════════════════════════════════

#[test]
fn word_equivalence_simple() {
    let ts_word = ts().parse_word("hello").unwrap();
    let re2c_word = re2c_word("hello");
    assert!(
        ts_word.semantic_eq(&re2c_word),
        "simple word mismatch:\n  ts:   {}\n  re2c: {}",
        serde_json::to_string(&ts_word).unwrap(),
        serde_json::to_string(&re2c_word).unwrap(),
    );
}

#[test]
fn word_equivalence_compound() {
    let ts_word = ts().parse_word("ice+cream").unwrap();
    let re2c_word = re2c_word("ice+cream");
    assert!(
        ts_word.semantic_eq(&re2c_word),
        "compound word mismatch:\n  ts:   {}\n  re2c: {}",
        serde_json::to_string(&ts_word).unwrap(),
        serde_json::to_string(&re2c_word).unwrap(),
    );
}

#[test]
fn word_equivalence_lengthening() {
    let ts_word = ts().parse_word("no::").unwrap();
    let re2c_word = re2c_word("no::");
    assert!(
        ts_word.semantic_eq(&re2c_word),
        "lengthening mismatch:\n  ts:   {}\n  re2c: {}",
        serde_json::to_string(&ts_word).unwrap(),
        serde_json::to_string(&re2c_word).unwrap(),
    );
}

#[test]
fn word_equivalence_form_marker() {
    let ts_word = ts().parse_word("mama@f").unwrap();
    let re2c_word = re2c_word("mama@f");
    assert!(
        ts_word.semantic_eq(&re2c_word),
        "form marker mismatch:\n  ts:   {}\n  re2c: {}",
        serde_json::to_string(&ts_word).unwrap(),
        serde_json::to_string(&re2c_word).unwrap(),
    );
}

#[test]
fn word_equivalence_shortening() {
    let ts_word = ts().parse_word("(be)cause").unwrap();
    let re2c_word = re2c_word("(be)cause");
    assert!(
        ts_word.semantic_eq(&re2c_word),
        "shortening mismatch:\n  ts:   {}\n  re2c: {}",
        serde_json::to_string(&ts_word).unwrap(),
        serde_json::to_string(&re2c_word).unwrap(),
    );
}

#[test]
fn word_equivalence_filler() {
    let ts_word = ts().parse_word("&-um").unwrap();
    let re2c_word = re2c_word("&-um");
    assert!(
        ts_word.semantic_eq(&re2c_word),
        "filler mismatch:\n  ts:   {}\n  re2c: {}",
        serde_json::to_string(&ts_word).unwrap(),
        serde_json::to_string(&re2c_word).unwrap(),
    );
}

#[test]
fn word_equivalence_lang_suffix() {
    let ts_word = ts().parse_word("hao3@s:zho").unwrap();
    let re2c_word = re2c_word("hao3@s:zho");
    assert!(
        ts_word.semantic_eq(&re2c_word),
        "lang suffix mismatch:\n  ts:   {}\n  re2c: {}",
        serde_json::to_string(&ts_word).unwrap(),
        serde_json::to_string(&re2c_word).unwrap(),
    );
}

// ═══════════════════════════════════════════════════════════════
// Main tier structure verification
// ═══════════════════════════════════════════════════════════════

#[test]
fn main_tier_retrace_structure() {
    // Verify our parser produces Retrace for "the the [/] dog ."
    let mt = talkbank_re2c_parser::parser::parse_main_tier("*CHI:\tthe the [/] dog .\n").unwrap();
    let has_retrace = mt
        .tier_body
        .contents
        .iter()
        .any(|c| matches!(c, talkbank_re2c_parser::ast::ContentItem::Retrace(_)));
    assert!(
        has_retrace,
        "expected Retrace in: {:?}",
        mt.tier_body.contents
    );
}

#[test]
fn main_tier_equivalence_simple() {
    let input = "*CHI:\thello world .\n";
    let ts_mt = ts().parse_main_tier(input).unwrap();
    let re2c_parsed = talkbank_re2c_parser::parser::parse_main_tier(input).unwrap();
    let re2c_mt = talkbank_re2c_parser::convert::main_tier_to_model(&re2c_parsed);
    assert!(
        ts_mt.semantic_eq(&re2c_mt),
        "simple main tier mismatch:\n  ts:   {}\n  re2c: {}",
        serde_json::to_string(&ts_mt).unwrap(),
        serde_json::to_string(&re2c_mt).unwrap(),
    );
}

#[test]
fn main_tier_equivalence_retrace() {
    let input = "*CHI:\tthe the [/] dog .\n";
    let ts_mt = ts().parse_main_tier(input).unwrap();
    let re2c_parsed = talkbank_re2c_parser::parser::parse_main_tier(input).unwrap();
    let re2c_mt = talkbank_re2c_parser::convert::main_tier_to_model(&re2c_parsed);
    assert!(
        ts_mt.semantic_eq(&re2c_mt),
        "retrace main tier mismatch:\n  ts:   {}\n  re2c: {}",
        serde_json::to_string(&ts_mt).unwrap(),
        serde_json::to_string(&re2c_mt).unwrap(),
    );
}

#[test]
fn main_tier_equivalence_compound() {
    let input = "*CHI:\tI want ice+cream .\n";
    let ts_mt = ts().parse_main_tier(input).unwrap();
    let re2c_parsed = talkbank_re2c_parser::parser::parse_main_tier(input).unwrap();
    let re2c_mt = talkbank_re2c_parser::convert::main_tier_to_model(&re2c_parsed);
    assert!(
        ts_mt.semantic_eq(&re2c_mt),
        "compound main tier mismatch:\n  ts:   {}\n  re2c: {}",
        serde_json::to_string(&ts_mt).unwrap(),
        serde_json::to_string(&re2c_mt).unwrap(),
    );
}

#[test]
fn main_tier_equivalence_event() {
    let input = "*CHI:\t&=laughs .\n";
    let ts_mt = ts().parse_main_tier(input).unwrap();
    let re2c_parsed = talkbank_re2c_parser::parser::parse_main_tier(input).unwrap();
    let re2c_mt = talkbank_re2c_parser::convert::main_tier_to_model(&re2c_parsed);
    assert!(
        ts_mt.semantic_eq(&re2c_mt),
        "event main tier mismatch:\n  ts:   {}\n  re2c: {}",
        serde_json::to_string(&ts_mt).unwrap(),
        serde_json::to_string(&re2c_mt).unwrap(),
    );
}

#[test]
fn main_tier_equivalence_pause() {
    let input = "*CHI:\tI (.) want cookies .\n";
    let ts_mt = ts().parse_main_tier(input).unwrap();
    let re2c_parsed = talkbank_re2c_parser::parser::parse_main_tier(input).unwrap();
    let re2c_mt = talkbank_re2c_parser::convert::main_tier_to_model(&re2c_parsed);
    assert!(
        ts_mt.semantic_eq(&re2c_mt),
        "pause main tier mismatch:\n  ts:   {}\n  re2c: {}",
        serde_json::to_string(&ts_mt).unwrap(),
        serde_json::to_string(&re2c_mt).unwrap(),
    );
}

#[test]
fn main_tier_equivalence_trailing_off() {
    let input = "*CHI:\tI was going to the +...\n";
    let ts_mt = ts().parse_main_tier(input).unwrap();
    let re2c_parsed = talkbank_re2c_parser::parser::parse_main_tier(input).unwrap();
    let re2c_mt = talkbank_re2c_parser::convert::main_tier_to_model(&re2c_parsed);
    assert!(
        ts_mt.semantic_eq(&re2c_mt),
        "trailing off mismatch:\n  ts:   {}\n  re2c: {}",
        serde_json::to_string(&ts_mt).unwrap(),
        serde_json::to_string(&re2c_mt).unwrap(),
    );
}

#[test]
fn mor_tier_equivalence() {
    let input = "pro|I v|want n|cookie-PL .\n";
    let errors = talkbank_model::errors::ErrorCollector::new();
    let ts_result = ts().parse_mor_tier_fragment(input, 0, &errors);
    let re2c_parsed = talkbank_re2c_parser::parser::parse_mor_tier(input);
    let re2c_tier = talkbank_model::model::MorTier::from(&re2c_parsed);
    if let talkbank_model::ParseOutcome::Parsed(ts_tier) = ts_result {
        assert!(
            ts_tier.semantic_eq(&re2c_tier),
            "mor tier mismatch:\n  ts:   {}\n  re2c: {}",
            serde_json::to_string(&ts_tier).unwrap(),
            serde_json::to_string(&re2c_tier).unwrap(),
        );
    } else {
        panic!("ts rejected mor tier");
    }
}

#[test]
fn gra_tier_equivalence() {
    let input = "1|2|SUBJ 2|0|ROOT 3|2|OBJ\n";
    let errors = talkbank_model::errors::ErrorCollector::new();
    let ts_result = ts().parse_gra_tier_fragment(input, 0, &errors);
    let re2c_parsed = talkbank_re2c_parser::parser::parse_gra_tier(input);
    let re2c_tier = talkbank_model::model::GraTier::from(&re2c_parsed);
    if let talkbank_model::ParseOutcome::Parsed(ts_tier) = ts_result {
        assert!(
            ts_tier.semantic_eq(&re2c_tier),
            "gra tier mismatch:\n  ts:   {}\n  re2c: {}",
            serde_json::to_string(&ts_tier).unwrap(),
            serde_json::to_string(&re2c_tier).unwrap(),
        );
    } else {
        panic!("ts rejected gra tier");
    }
}

// ═══════════════════════════════════════════════════════════════
// Full file equivalence
// ═══════════════════════════════════════════════════════════════

#[test]
fn file_equivalence_basic_conversation() {
    let path = format!(
        "{}/corpus/reference/core/basic-conversation.cha",
        env!("CARGO_MANIFEST_DIR").replace("/crates/talkbank-re2c-parser", "")
    );
    let content = std::fs::read_to_string(&path).expect("read basic-conversation.cha");

    let ts_file = ts().parse_chat_file(&content).expect("ts parse");
    let re2c_parsed = talkbank_re2c_parser::parser::parse_chat_file(&content);
    let re2c_file = talkbank_model::model::ChatFile::from(&re2c_parsed);

    if !ts_file.semantic_eq(&re2c_file) {
        // Show per-line comparison
        eprintln!("FILE MISMATCH: basic-conversation.cha");
        eprintln!("  ts lines: {}", ts_file.lines.len());
        eprintln!("  re2c lines: {}", re2c_file.lines.len());

        let ts_json = serde_json::to_string_pretty(&ts_file).unwrap();
        let re2c_json = serde_json::to_string_pretty(&re2c_file).unwrap();

        // Write to temp files for diff
        std::fs::write("/tmp/ts_output.json", &ts_json).ok();
        std::fs::write("/tmp/re2c_output.json", &re2c_json).ok();
        eprintln!("  diff /tmp/ts_output.json /tmp/re2c_output.json");

        panic!("semantic mismatch — see /tmp/*.json for details");
    }
}

#[test]
fn file_equivalence_reference_corpus_core() {
    let base = format!(
        "{}/corpus/reference/core",
        env!("CARGO_MANIFEST_DIR").replace("/crates/talkbank-re2c-parser", "")
    );
    let base_path = std::path::Path::new(&base);
    if !base_path.exists() {
        return;
    }

    let mut passed = 0;
    let mut failed = Vec::new();

    for entry in std::fs::read_dir(base_path).unwrap() {
        let path = entry.unwrap().path();
        if !path.extension().is_some_and(|e| e == "cha") {
            continue;
        }
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let content = std::fs::read_to_string(&path).unwrap();

        let ts_file = ts().parse_chat_file(&content).unwrap();
        let re2c_parsed = talkbank_re2c_parser::parser::parse_chat_file(&content);
        let re2c_file = talkbank_model::model::ChatFile::from(&re2c_parsed);

        if ts_file.semantic_eq(&re2c_file) {
            passed += 1;
        } else {
            if failed.is_empty() {
                // Write first failure for debugging
                std::fs::write(
                    "/tmp/ts_output.json",
                    serde_json::to_string_pretty(&ts_file).unwrap(),
                )
                .ok();
                std::fs::write(
                    "/tmp/re2c_output.json",
                    serde_json::to_string_pretty(&re2c_file).unwrap(),
                )
                .ok();
            }
            failed.push(filename);
        }
    }

    eprintln!(
        "core/ equivalence: {passed} passed, {} failed",
        failed.len()
    );
    for f in &failed {
        eprintln!("  FAIL: {f}");
    }
    assert!(
        failed.is_empty(),
        "{} files failed equivalence",
        failed.len()
    );
}

#[test]
fn file_equivalence_reference_corpus_content() {
    let base = format!(
        "{}/corpus/reference/content",
        env!("CARGO_MANIFEST_DIR").replace("/crates/talkbank-re2c-parser", "")
    );
    let base_path = std::path::Path::new(&base);
    if !base_path.exists() {
        return;
    }

    let mut passed = 0;
    let mut failed = Vec::new();

    for entry in std::fs::read_dir(base_path).unwrap() {
        let path = entry.unwrap().path();
        if !path.extension().is_some_and(|e| e == "cha") {
            continue;
        }
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let content = std::fs::read_to_string(&path).unwrap();

        let ts_file = ts().parse_chat_file(&content).unwrap();
        let re2c_parsed = talkbank_re2c_parser::parser::parse_chat_file(&content);
        let re2c_file = talkbank_model::model::ChatFile::from(&re2c_parsed);

        if ts_file.semantic_eq(&re2c_file) {
            passed += 1;
        } else {
            if failed.is_empty() {
                std::fs::write(
                    "/tmp/ts_output.json",
                    serde_json::to_string_pretty(&ts_file).unwrap(),
                )
                .ok();
                std::fs::write(
                    "/tmp/re2c_output.json",
                    serde_json::to_string_pretty(&re2c_file).unwrap(),
                )
                .ok();
            }
            failed.push(filename);
        }
    }

    eprintln!(
        "content/ equivalence: {passed} passed, {} failed",
        failed.len()
    );
    for f in &failed {
        eprintln!("  FAIL: {f}");
    }
    assert!(
        failed.is_empty(),
        "{} files failed equivalence",
        failed.len()
    );
}

#[test]
fn file_equivalence_reference_corpus_annotation() {
    let base = format!(
        "{}/corpus/reference/annotation",
        env!("CARGO_MANIFEST_DIR").replace("/crates/talkbank-re2c-parser", "")
    );
    let base_path = std::path::Path::new(&base);
    if !base_path.exists() {
        return;
    }

    let mut passed = 0;
    let mut failed = Vec::new();

    for entry in std::fs::read_dir(base_path).unwrap() {
        let path = entry.unwrap().path();
        if !path.extension().is_some_and(|e| e == "cha") {
            continue;
        }
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let content = std::fs::read_to_string(&path).unwrap();

        let ts_file = ts().parse_chat_file(&content).unwrap();
        let re2c_parsed = talkbank_re2c_parser::parser::parse_chat_file(&content);
        let re2c_file = talkbank_model::model::ChatFile::from(&re2c_parsed);

        if ts_file.semantic_eq(&re2c_file) {
            passed += 1;
        } else {
            if failed.is_empty() {
                std::fs::write(
                    "/tmp/ts_output.json",
                    serde_json::to_string_pretty(&ts_file).unwrap(),
                )
                .ok();
                std::fs::write(
                    "/tmp/re2c_output.json",
                    serde_json::to_string_pretty(&re2c_file).unwrap(),
                )
                .ok();
            }
            failed.push(filename);
        }
    }

    eprintln!(
        "annotation/ equivalence: {passed} passed, {} failed",
        failed.len()
    );
    for f in &failed {
        eprintln!("  FAIL: {f}");
    }
    assert!(
        failed.is_empty(),
        "{} files failed equivalence",
        failed.len()
    );
}

macro_rules! corpus_equivalence_test {
    ($name:ident, $dir:expr) => {
        #[test]
        fn $name() {
            let base = format!(
                "{}/corpus/reference/{}",
                env!("CARGO_MANIFEST_DIR").replace("/crates/talkbank-re2c-parser", ""),
                $dir
            );
            let base_path = std::path::Path::new(&base);
            if !base_path.exists() {
                return;
            }
            let mut passed = 0;
            let mut failed = Vec::new();
            for entry in std::fs::read_dir(base_path).unwrap() {
                let path = entry.unwrap().path();
                if !path.extension().is_some_and(|e| e == "cha") {
                    continue;
                }
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                let content = std::fs::read_to_string(&path).unwrap();
                let ts_file = ts().parse_chat_file(&content).unwrap();
                let re2c_parsed = talkbank_re2c_parser::parser::parse_chat_file(&content);
                let re2c_file = talkbank_model::model::ChatFile::from(&re2c_parsed);
                if ts_file.semantic_eq(&re2c_file) {
                    passed += 1;
                } else {
                    if failed.is_empty() {
                        std::fs::write(
                            "/tmp/ts_output.json",
                            serde_json::to_string_pretty(&ts_file).unwrap(),
                        )
                        .ok();
                        std::fs::write(
                            "/tmp/re2c_output.json",
                            serde_json::to_string_pretty(&re2c_file).unwrap(),
                        )
                        .ok();
                    }
                    failed.push(filename);
                }
            }
            eprintln!(
                "{} equivalence: {passed} passed, {} failed",
                $dir,
                failed.len()
            );
            for f in &failed {
                eprintln!("  FAIL: {f}");
            }
            assert!(
                failed.is_empty(),
                "{} files failed equivalence",
                failed.len()
            );
        }
    };
}

corpus_equivalence_test!(file_equivalence_reference_corpus_tiers, "tiers");
corpus_equivalence_test!(file_equivalence_reference_corpus_ca, "ca");
corpus_equivalence_test!(file_equivalence_reference_corpus_languages, "languages");
corpus_equivalence_test!(
    file_equivalence_reference_corpus_word_features,
    "word-features"
);

#[test]
#[ignore]
fn study_retrace_cha_dep_tiers() {
    let base = format!(
        "{}/corpus/reference/annotation",
        env!("CARGO_MANIFEST_DIR").replace("/crates/talkbank-re2c-parser", "")
    );
    let content = std::fs::read_to_string(format!("{base}/retrace.cha")).unwrap();
    let re2c_parsed = talkbank_re2c_parser::parser::parse_chat_file(&content);
    let re2c_file = talkbank_model::model::ChatFile::from(&re2c_parsed);
    for (i, line) in re2c_file.lines.iter().enumerate() {
        if let talkbank_model::model::Line::Utterance(u) = line {
            let dep_types: Vec<_> = u
                .dependent_tiers
                .iter()
                .map(|d| format!("{:?}", std::mem::discriminant(d)))
                .collect();
            eprintln!("Line {i}: {} deps: {dep_types:?}", u.dependent_tiers.len());
        }
    }
}

/// Parse a word using our re2c parser and convert to model Word.
fn re2c_word(input: &str) -> talkbank_model::model::Word {
    let parsed = talkbank_re2c_parser::parser::parse_word(input).expect("re2c parse_word");
    talkbank_re2c_parser::convert::word_from_parsed(&parsed)
}

#[test]
#[ignore]
fn study_word_compound() {
    let w = ts().parse_word("ice+cream").unwrap();
    eprintln!("{}", serde_json::to_string_pretty(&w).unwrap());
}

#[test]
#[ignore]
fn study_word_form_marker() {
    let w = ts().parse_word("mama@f").unwrap();
    eprintln!("{}", serde_json::to_string_pretty(&w).unwrap());
}

#[test]
#[ignore]
fn study_word_lengthening() {
    let w = ts().parse_word("no::").unwrap();
    eprintln!("{}", serde_json::to_string_pretty(&w).unwrap());
}

#[test]
#[ignore]
fn study_mor_tier() {
    let errors = talkbank_model::errors::ErrorCollector::new();
    let tier = ts().parse_mor_tier_fragment("pro|I v|want n|cookie-PL .\n", 0, &errors);
    if let talkbank_model::ParseOutcome::Parsed(t) = tier {
        eprintln!("{}", serde_json::to_string_pretty(&t).unwrap());
    } else {
        eprintln!("REJECTED");
    }
}

#[test]
#[ignore]
fn study_gra_tier() {
    let errors = talkbank_model::errors::ErrorCollector::new();
    let tier = ts().parse_gra_tier_fragment("1|2|SUBJ 2|0|ROOT 3|2|OBJ .\n", 0, &errors);
    if let talkbank_model::ParseOutcome::Parsed(t) = tier {
        eprintln!("{}", serde_json::to_string_pretty(&t).unwrap());
    } else {
        eprintln!("REJECTED");
    }
}
