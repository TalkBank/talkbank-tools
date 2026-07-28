use crate::common::assert_completed_without_errors;
use batchalign::api::ReleasedCommand;
use batchalign::options::{CommandOptions, CommonOptions, MorphotagOptions};
use batchalign::worker::InferTask;

use crate::ml_golden::golden::fixtures::{
    ENG_DISFLUENCY_PARITY, ENG_MULTI_UTT, ENG_RETOKENIZE, ENG_SIMPLE,
    ITA_MULTI_WORD_UTTERANCES, ITA_SINGLE_WORD_UTTERANCES, SPA_SIMPLE,
};
use crate::ml_golden::golden::helpers::{
    assert_golden_snapshot, find_mor_line_for, has_gra_tier, has_mor_tier, parse_output,
    require_direct_session_warmed,
};

fn morphotag_options(override_media_cache: bool, retokenize: bool) -> CommandOptions {
    CommandOptions::Morphotag(MorphotagOptions {
        common: CommonOptions {
            override_media_cache,
            ..CommonOptions::default()
        },
        retokenize,

        ..Default::default()
    })
}

#[tokio::test]
async fn golden_morphotag_eng_simple() {
    let Some(jobs) = require_direct_session_warmed(
        InferTask::Morphosyntax,
        ReleasedCommand::Morphotag,
        "eng",
        "Direct session does not support morphosyntax infer",
    )
    .await
    else {
        return;
    };

    let (info, results) = jobs
        .submit_content_job(
            ReleasedCommand::Morphotag,
            "eng",
            "eng_simple.cha",
            ENG_SIMPLE,
            morphotag_options(true, false),
        )
        .await;

    assert_completed_without_errors("morphotag_eng_simple", &info, &results);
    let output = &results[0].content;
    let file = parse_output(output, "morphotag_eng_simple");
    assert!(has_mor_tier(&file));
    assert!(has_gra_tier(&file));
    assert_golden_snapshot!("morphotag_eng_simple", output);
}

#[tokio::test]
async fn golden_morphotag_eng_multi_utt() {
    let Some(jobs) = require_direct_session_warmed(
        InferTask::Morphosyntax,
        ReleasedCommand::Morphotag,
        "eng",
        "Direct session does not support morphosyntax infer",
    )
    .await
    else {
        return;
    };

    let (info, results) = jobs
        .submit_content_job(
            ReleasedCommand::Morphotag,
            "eng",
            "eng_multi_utt.cha",
            ENG_MULTI_UTT,
            morphotag_options(true, false),
        )
        .await;

    assert_completed_without_errors("morphotag_eng_multi_utt", &info, &results);
    assert_golden_snapshot!("morphotag_eng_multi_utt", &results[0].content);
}

#[tokio::test]
async fn golden_morphotag_with_cache() {
    let Some(jobs) = require_direct_session_warmed(
        InferTask::Morphosyntax,
        ReleasedCommand::Morphotag,
        "eng",
        "Direct session does not support morphosyntax infer",
    )
    .await
    else {
        return;
    };

    let (info1, results1) = jobs
        .submit_content_job(
            ReleasedCommand::Morphotag,
            "eng",
            "cache_test.cha",
            ENG_SIMPLE,
            morphotag_options(false, false),
        )
        .await;
    assert_completed_without_errors("morphotag_with_cache_cold", &info1, &results1);

    let (info2, results2) = jobs
        .submit_content_job(
            ReleasedCommand::Morphotag,
            "eng",
            "cache_test.cha",
            ENG_SIMPLE,
            morphotag_options(false, false),
        )
        .await;
    assert_completed_without_errors("morphotag_with_cache_warm", &info2, &results2);

    assert_eq!(results1[0].content, results2[0].content);
}

#[tokio::test]
async fn golden_morphotag_spa_simple() {
    let Some(jobs) = require_direct_session_warmed(
        InferTask::Morphosyntax,
        ReleasedCommand::Morphotag,
        "spa",
        "Direct session does not support morphosyntax infer",
    )
    .await
    else {
        return;
    };

    let (info, results) = jobs
        .submit_content_job(
            ReleasedCommand::Morphotag,
            "spa",
            "spa_simple.cha",
            SPA_SIMPLE,
            morphotag_options(true, false),
        )
        .await;

    if info.status == batchalign::api::JobStatus::Failed {
        eprintln!("SKIP: Spanish morphotag failed (model likely not downloaded)");
        return;
    }

    assert_completed_without_errors("morphotag_spa_simple", &info, &results);
    let file = parse_output(&results[0].content, "morphotag_spa_simple");
    assert!(has_mor_tier(&file));
    assert!(has_gra_tier(&file));
    assert_golden_snapshot!("morphotag_spa_simple", &results[0].content);
}

#[tokio::test]
async fn golden_morphotag_retokenize_eng() {
    let Some(jobs) = require_direct_session_warmed(
        InferTask::Morphosyntax,
        ReleasedCommand::Morphotag,
        "eng",
        "Direct session does not support morphosyntax infer",
    )
    .await
    else {
        return;
    };

    let (info, results) = jobs
        .submit_content_job(
            ReleasedCommand::Morphotag,
            "eng",
            "eng_retokenize.cha",
            ENG_RETOKENIZE,
            morphotag_options(true, true),
        )
        .await;

    assert_completed_without_errors("morphotag_retokenize_eng", &info, &results);
    let file = parse_output(&results[0].content, "morphotag_retokenize_eng");
    assert!(has_mor_tier(&file));
    assert!(has_gra_tier(&file));
    assert_golden_snapshot!("morphotag_retokenize_eng", &results[0].content);
}

#[tokio::test]
async fn morphotag_disfluency_preserves_thats_subject_and_copula() {
    let Some(jobs) = require_direct_session_warmed(
        InferTask::Morphosyntax,
        ReleasedCommand::Morphotag,
        "eng",
        "Direct session does not support morphosyntax infer",
    )
    .await
    else {
        return;
    };

    let (info, results) = jobs
        .submit_content_job(
            ReleasedCommand::Morphotag,
            "eng",
            "eng_disfluency.cha",
            ENG_DISFLUENCY_PARITY,
            morphotag_options(true, false),
        )
        .await;

    assert_completed_without_errors(
        "morphotag_disfluency_preserves_thats_subject_and_copula",
        &info,
        &results,
    );
    assert_eq!(results.len(), 1);

    let mmhmm_mor = find_mor_line_for(&results[0].content, "mm-hmm that's right")
        .expect("expected %mor line for mm-hmm that's right");
    assert!(
        mmhmm_mor.contains("pron|that-Dem~aux|be-Fin-Ind-Pres-S3"),
        "expected explicit subject+copula analysis for \"that's right\", got: {mmhmm_mor}"
    );
    assert!(
        !mmhmm_mor.contains("aux|that-Fin-Ind-Pres-S3"),
        "unexpected collapsed auxiliary-only analysis for \"that's right\": {mmhmm_mor}"
    );
}

#[tokio::test]
async fn golden_morphotag_cache_is_faster() {
    let Some(jobs) = require_direct_session_warmed(
        InferTask::Morphosyntax,
        ReleasedCommand::Morphotag,
        "eng",
        "Direct session does not support morphosyntax infer",
    )
    .await
    else {
        return;
    };

    let start1 = std::time::Instant::now();
    let (info1, results1) = jobs
        .submit_content_job(
            ReleasedCommand::Morphotag,
            "eng",
            "cache_speed.cha",
            ENG_SIMPLE,
            morphotag_options(false, false),
        )
        .await;
    let elapsed1 = start1.elapsed();
    assert_completed_without_errors("morphotag_cache_is_faster_cold", &info1, &results1);

    let start2 = std::time::Instant::now();
    let (info2, results2) = jobs
        .submit_content_job(
            ReleasedCommand::Morphotag,
            "eng",
            "cache_speed.cha",
            ENG_SIMPLE,
            morphotag_options(false, false),
        )
        .await;
    let elapsed2 = start2.elapsed();
    assert_completed_without_errors("morphotag_cache_is_faster_warm", &info2, &results2);

    assert_eq!(results1[0].content, results2[0].content);
    eprintln!(
        "Cache timing: cold={:?}, warm={:?} (speedup: {:.1}x)",
        elapsed1,
        elapsed2,
        elapsed1.as_secs_f64() / elapsed2.as_secs_f64()
    );
    if elapsed1.as_secs_f64() > 1.0 {
        assert!(elapsed2 < elapsed1);
    }
}

/// Italian single-word utterances must not be shredded into invented verbs.
///
/// Stanza's Italian MWT treats multi-word tokens as an open class and, on
/// utterances with no syntactic context, split roughly a third of real corpus
/// words into a nonexistent verb plus a clitic: `attenzione` -> *attenzi* +
/// *ne*, `cavallo` -> *cava* + *lo*, `gallina` -> *galli* + *na* (where "na"
/// is not even a clitic), `mucche` -> *mu* + *cce* + *he*. That corrupted
/// `%mor` with verbs that do not exist in Italian, not merely the `%gra`
/// relation, and it destroyed plural lemmas along the way.
///
/// Measured 2026-07-28 on stanza 1.13.0 against real single-word utterances
/// pulled from the CHILDES Italian corpora. Fixed by suppressing MWT expansion
/// on single-word utterances outside Italian's closed MWT classes; see
/// `batchalign/inference/_italian_mwt.py`.
///
/// `eccolo` is the deliberate control: `ecco`+clitic IS a closed-class Italian
/// multi-word token, so it must STILL split. A fix that silenced it too would
/// pass a naive "no invented verbs" assertion while quietly losing real
/// analysis.
#[tokio::test]
async fn golden_morphotag_ita_single_word_utterances_are_not_split() {
    let Some(jobs) = require_direct_session_warmed(
        InferTask::Morphosyntax,
        ReleasedCommand::Morphotag,
        "ita",
        "Direct session does not support morphosyntax infer",
    )
    .await
    else {
        return;
    };

    let (info, results) = jobs
        .submit_content_job(
            ReleasedCommand::Morphotag,
            "ita",
            "ita_single_word.cha",
            ITA_SINGLE_WORD_UTTERANCES,
            morphotag_options(true, false),
        )
        .await;

    assert_completed_without_errors("morphotag_ita_single_word", &info, &results);
    let output = &results[0].content;
    let file = parse_output(output, "morphotag_ita_single_word");
    assert!(has_mor_tier(&file));
    assert!(has_gra_tier(&file));

    // Each noun must be tagged as a noun, with no clitic split.
    for (utterance, expected_lemma) in [
        ("attenzione", "attenzione"),
        ("macchine", "macchina"),
        ("gallina", "gallina"),
        ("cavallo", "cavallo"),
        ("mucche", "mucca"),
        ("persone", "persona"),
    ] {
        let mor = find_mor_line_for(output, utterance)
            .unwrap_or_else(|| panic!("no %mor tier for {utterance:?}"));
        assert!(
            mor.contains("noun|"),
            "{utterance:?} must be tagged a noun, got {mor:?}"
        );
        assert!(
            !mor.contains('~'),
            "{utterance:?} must not be split into a multi-word token, got {mor:?}"
        );
        assert!(
            mor.contains(expected_lemma),
            "{utterance:?} should carry lemma {expected_lemma:?} (the split \
             destroyed plural lemmas), got {mor:?}"
        );
    }

    // Control: a genuine closed-class MWT must still expand.
    let ecco = find_mor_line_for(output, "eccolo").expect("no %mor tier for eccolo");
    assert!(
        ecco.contains('~'),
        "eccolo is a real ecco+clitic multi-word token and must still split, got {ecco:?}"
    );
}

/// Italian multi-word utterances: genuine MWTs expand, verb forms do not.
///
/// The single-word suppression must not leak into multi-word context, where
/// stanza analyzes these same nouns correctly and where preposition+article
/// contractions carry real linguistic information.
///
/// `dai` appears twice on purpose, in both of its readings: as the 2sg verb
/// *dare* ("dai il libro a me") and as the contraction *da* + *il* ("vieni dai
/// bambini"). `hai` is subjectless because Italian is pro-drop, which is the
/// normal spoken form and the context where a mis-analysis is most likely.
#[tokio::test]
async fn golden_morphotag_ita_multi_word_keeps_genuine_mwts() {
    let Some(jobs) = require_direct_session_warmed(
        InferTask::Morphosyntax,
        ReleasedCommand::Morphotag,
        "ita",
        "Direct session does not support morphosyntax infer",
    )
    .await
    else {
        return;
    };

    let (info, results) = jobs
        .submit_content_job(
            ReleasedCommand::Morphotag,
            "ita",
            "ita_multi_word.cha",
            ITA_MULTI_WORD_UTTERANCES,
            morphotag_options(true, false),
        )
        .await;

    assert_completed_without_errors("morphotag_ita_multi_word", &info, &results);
    let output = &results[0].content;
    // Parsed for its side effect: parse_output asserts the output is valid CHAT.
    let _file = parse_output(output, "morphotag_ita_multi_word");

    // Preposition + article contractions must still expand.
    let contractions = find_mor_line_for(output, "vado")
        .expect("no %mor tier for the contraction utterance");
    assert!(
        contractions.matches('~').count() >= 2,
        "alla and della are genuine contractions and must both expand, got {contractions:?}"
    );

    // Pro-drop 2sg `hai` is never a contraction in Italian: no `ha` + `i`.
    let hai = find_mor_line_for(output, "hai").expect("no %mor tier for hai");
    assert!(
        hai.contains("avere"),
        "hai is 2sg of avere and must not be split into ha + i, got {hai:?}"
    );

    // Nouns in context keep their correct lemmas.
    let persone = find_mor_line_for(output, "sono").expect("no %mor tier for persone");
    assert!(
        persone.contains("persona"),
        "persone should lemmatize to persona in context, got {persone:?}"
    );
}
