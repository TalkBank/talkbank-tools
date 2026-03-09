use talkbank_model::ErrorCollector;
use talkbank_model::ParseOutcome;
use talkbank_model::dependent_tier::DependentTier;
use talkbank_model::model::ParseHealthTier;

use super::dispatch::TierParseResult;
use super::helpers::{dependent_tier_label_bytes, split_tier_label_and_content};
use super::{
    classify_dependent_tier_parse_health, parse_dependent_tier_impl, parse_dependent_tier_internal,
};

#[test]
fn parses_known_tier_with_byte_dispatch() {
    let errors = ErrorCollector::new();
    let tier = parse_dependent_tier_impl("%com:\thello world", 0, &errors);
    assert!(matches!(tier, ParseOutcome::Parsed(DependentTier::Com(_))));
    assert!(errors.is_empty());
}

#[test]
fn parses_unknown_tier_as_user_defined() {
    let errors = ErrorCollector::new();
    let tier = parse_dependent_tier_impl("%xfoo:\tbar baz", 0, &errors);
    assert!(matches!(
        tier,
        ParseOutcome::Parsed(DependentTier::UserDefined(_))
    ));
    assert!(errors.is_empty());
}

#[test]
fn rejects_invalid_dependent_tier_format() {
    let errors = ErrorCollector::new();
    let tier = parse_dependent_tier_impl("%mor no_tab_separator", 0, &errors);
    assert!(tier.is_rejected());
    assert!(!errors.is_empty());
}

#[test]
fn rejects_empty_simple_text_tier_content() {
    let errors = ErrorCollector::new();
    let tier = parse_dependent_tier_impl("%alt:\t", 0, &errors);
    assert!(tier.is_rejected());
    assert!(!errors.is_empty());
}

#[test]
fn rejects_empty_user_defined_tier_content() {
    let errors = ErrorCollector::new();
    let tier = parse_dependent_tier_impl("%xfoo:\t", 0, &errors);
    assert!(tier.is_rejected());
    assert!(!errors.is_empty());
}

#[test]
fn parses_non_x_unknown_tier_as_unsupported() {
    let errors = ErrorCollector::new();
    let tier = parse_dependent_tier_impl("%foo:\tsome content", 0, &errors);
    assert!(matches!(
        tier,
        ParseOutcome::Parsed(DependentTier::Unsupported(_))
    ));
    assert!(errors.is_empty());
}

#[test]
fn rejects_empty_unsupported_tier_content() {
    let errors = ErrorCollector::new();
    let tier = parse_dependent_tier_impl("%foo:\t", 0, &errors);
    assert!(tier.is_rejected());
    assert!(!errors.is_empty());
}

#[test]
fn classifies_parse_health_from_well_formed_and_malformed_labels() {
    assert_eq!(
        classify_dependent_tier_parse_health("%mor:\tpro|I v|go"),
        Some(ParseHealthTier::Mor)
    );
    assert_eq!(
        classify_dependent_tier_parse_health("%gra no_tab_separator"),
        Some(ParseHealthTier::Gra)
    );
    assert_eq!(
        classify_dependent_tier_parse_health("%xmod:\tphonological content"),
        Some(ParseHealthTier::Mod)
    );
    assert_eq!(classify_dependent_tier_parse_health("%foo:\tbar"), None);
    assert_eq!(classify_dependent_tier_parse_health("*CHI:\thello ."), None);
}

#[test]
fn split_tier_label_and_content_valid() {
    let (label, content, offset) = split_tier_label_and_content("%mor:\tpro|I").unwrap();
    assert_eq!(label, "mor");
    assert_eq!(content, "pro|I");
    assert_eq!(offset, 6);
}

#[test]
fn split_tier_label_rejects_missing_percent() {
    assert!(split_tier_label_and_content("mor:\tcontent").is_none());
}

#[test]
fn split_tier_label_rejects_empty_label() {
    assert!(split_tier_label_and_content("%:\tcontent").is_none());
}

#[test]
fn split_tier_label_rejects_no_tab() {
    assert!(split_tier_label_and_content("%mor:content").is_none());
}

#[test]
fn split_tier_label_content_offset_is_after_colon_tab() {
    let (_, _, offset) = split_tier_label_and_content("%gra:\t1|2|SUBJ").unwrap();
    assert_eq!(offset, 6);
}

#[test]
fn split_tier_label_long_label() {
    let (label, content, offset) =
        split_tier_label_and_content("%xmod:\tphonological content").unwrap();
    assert_eq!(label, "xmod");
    assert_eq!(content, "phonological content");
    assert_eq!(offset, 7);
}

#[test]
fn label_bytes_extracts_before_colon() {
    assert_eq!(
        dependent_tier_label_bytes("%mor:\tpro|I"),
        Some(b"mor" as &[u8])
    );
}

#[test]
fn label_bytes_extracts_before_tab() {
    assert_eq!(
        dependent_tier_label_bytes("%gra\tbad"),
        Some(b"gra" as &[u8])
    );
}

#[test]
fn label_bytes_extracts_before_space() {
    assert_eq!(
        dependent_tier_label_bytes("%mor bad"),
        Some(b"mor" as &[u8])
    );
}

#[test]
fn label_bytes_rejects_no_percent() {
    assert!(dependent_tier_label_bytes("mor:\tcontent").is_none());
}

#[test]
fn label_bytes_rejects_only_percent() {
    assert!(dependent_tier_label_bytes("%").is_none());
}

#[test]
fn label_bytes_rejects_percent_then_colon() {
    assert!(dependent_tier_label_bytes("%:").is_none());
}

#[test]
fn parses_mor_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%mor:\tpro|I v|go .", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Mor(_))
    ));
}

#[test]
fn parses_gra_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%gra:\t1|2|DET 2|0|ROOT 3|2|PUNCT", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Gra(_))
    ));
}

#[test]
fn parses_pho_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%pho:\thɛˈloʊ .", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Pho(_))
    ));
}

#[test]
fn parses_sin_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%sin:\thello .", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Sin(_))
    ));
}

#[test]
fn parses_exp_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%exp:\tpoints to dog", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Exp(_))
    ));
}

#[test]
fn parses_act_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%act:\tpoints to door", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Act(_))
    ));
}

#[test]
fn parses_cod_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%cod:\t$ABC", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Cod(_))
    ));
}

#[test]
fn parses_add_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%add:\tsituation note", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Add(_))
    ));
}

#[test]
fn parses_gpx_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%gpx:\tgesture description", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Gpx(_))
    ));
}

#[test]
fn parses_int_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%int:\tinterlinear gloss", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Int(_))
    ));
}

#[test]
fn parses_spa_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%spa:\t$IMIT", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Spa(_))
    ));
}

#[test]
fn parses_sit_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%sit:\tsituation note", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Sit(_))
    ));
}

#[test]
fn parses_wor_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%wor:\thello .", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Wor(_))
    ));
}

#[test]
fn parses_alt_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%alt:\talternative", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Alt(_))
    ));
}

#[test]
fn parses_tim_tier_cleanly() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%tim:\t00:01:23.456-00:01:25.789", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Clean(DependentTier::Tim(_))
    ));
}

#[test]
fn rejects_empty_tim_tier() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%tim:\t", 0, &errors);
    assert!(matches!(result, TierParseResult::Failed(None)));
}

#[test]
fn invalid_tier_format_reports_taint() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%mor bad_no_tab", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Failed(Some(ParseHealthTier::Mor))
    ));
}

#[test]
fn invalid_gra_tier_format_reports_gra_taint() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%gra bad_no_tab", 0, &errors);
    assert!(matches!(
        result,
        TierParseResult::Failed(Some(ParseHealthTier::Gra))
    ));
}

#[test]
fn invalid_unknown_tier_reports_no_taint() {
    let errors = ErrorCollector::new();
    let result = parse_dependent_tier_internal("%zzz bad_no_tab", 0, &errors);
    assert!(matches!(result, TierParseResult::Failed(None)));
}

#[test]
fn nonzero_offset_propagates_to_content_parser() {
    let errors = ErrorCollector::new();
    let tier = parse_dependent_tier_impl("%com:\thello world", 100, &errors);
    assert!(matches!(tier, ParseOutcome::Parsed(DependentTier::Com(_))));
    assert!(errors.is_empty());
}

#[test]
fn classifies_all_taintable_tiers() {
    assert_eq!(
        classify_dependent_tier_parse_health("%pho:\tcontent"),
        Some(ParseHealthTier::Pho)
    );
    assert_eq!(
        classify_dependent_tier_parse_health("%wor:\tcontent"),
        Some(ParseHealthTier::Wor)
    );
    assert_eq!(
        classify_dependent_tier_parse_health("%sin:\tcontent"),
        Some(ParseHealthTier::Sin)
    );
    assert_eq!(
        classify_dependent_tier_parse_health("%mod:\tcontent"),
        Some(ParseHealthTier::Mod)
    );
}

#[test]
fn classifies_malformed_taintable_tiers() {
    assert_eq!(
        classify_dependent_tier_parse_health("%pho no_tab"),
        Some(ParseHealthTier::Pho)
    );
    assert_eq!(
        classify_dependent_tier_parse_health("%wor no_tab"),
        Some(ParseHealthTier::Wor)
    );
    assert_eq!(
        classify_dependent_tier_parse_health("%sin no_tab"),
        Some(ParseHealthTier::Sin)
    );
}

#[test]
fn parses_all_simple_text_tiers() {
    let cases: Vec<(&str, fn(&DependentTier) -> bool)> = vec![
        ("%coh:\tcontent", |t| matches!(t, DependentTier::Coh(_))),
        ("%def:\tcontent", |t| matches!(t, DependentTier::Def(_))),
        ("%eng:\tcontent", |t| matches!(t, DependentTier::Eng(_))),
        ("%err:\tcontent", |t| matches!(t, DependentTier::Err(_))),
        ("%fac:\tcontent", |t| matches!(t, DependentTier::Fac(_))),
        ("%flo:\tcontent", |t| matches!(t, DependentTier::Flo(_))),
        ("%gls:\tcontent", |t| matches!(t, DependentTier::Gls(_))),
        ("%ort:\tcontent", |t| matches!(t, DependentTier::Ort(_))),
        ("%par:\tcontent", |t| matches!(t, DependentTier::Par(_))),
    ];

    for (label, variant_check) in cases {
        let errors = ErrorCollector::new();
        let result = parse_dependent_tier_internal(label, 0, &errors);
        match result {
            TierParseResult::Clean(ref tier) => {
                assert!(variant_check(tier), "Wrong variant for {label}");
            }
            other => panic!("Expected Clean for {label}, got {other:?}"),
        }
    }
}
