//! Server-side forced alignment orchestrator.
//!
//! Owns the full CHAT lifecycle for FA jobs:
//! parse → group → cache check → infer (audio chunks) → DP-align → inject →
//! postprocess → %wor → monotonicity/E704 → serialize.
//!
//! # Call path
//!
//! `batchalign-cli`/API submission
//! → `runner::dispatch_fa_infer`
//! → [`process_fa`]
//! → `crate::chat_ops::fa::{group_utterances, parse_fa_response, apply_fa_results}`
//! → FA worker transport adapter
//! → validation + serialization.
//!
//! # Key differences from morphosyntax/utseg/translate/coref
//!
//! - **Per-file, not cross-file**: Each file has its own audio, so no cross-file batching.
//! - **Multiple groups per file**: Utterances are grouped by time window; each group is one infer item.
//! - **Audio access**: Workers need the audio file path and time range, not just text.
//! - **DP alignment in Rust**: Model output is aligned to transcript words via Hirschberg.
//!
//! # Invariants for contributors
//!
//! - FA worker timestamps are chunk-relative; `parse_fa_response` must convert
//!   them to file-absolute ms with `audio_start_ms`.
//! - `apply_fa_results` ordering is load-bearing:
//!   inject → postprocess → utterance bullet update → `%wor` generation
//!   → monotonicity (E362) → same-speaker overlap enforcement (E704).
//! - Cache keys must include audio identity + time window + text + timing mode
//!   + engine; changing dimensions changes cache compatibility.

mod raw_evidence;
mod transport;

use crate::cache::CacheBackend;
use crate::chat_ops::fa::{
    WordTiming, apply_fa_results, cache_key, enforce_monotonicity, expand_bullets_for_edge_fillers,
    find_reusable_utterance_indices, group_utterances, has_reusable_wor_timing,
    refresh_existing_alignment, refresh_reusable_utterances, rescue_narrow_bullets,
    strip_wor_from_monotonicity_stripped_utterances,
};
use crate::chat_ops::{CacheKey, CacheTaskName};
use crate::params::{AudioContext, FaParams};
use crate::pipeline::PipelineServices;
use batchalign_transform::parse::{is_ca, is_dummy, is_no_align, parse_lenient};
use batchalign_transform::serialize::to_chat_string;
use batchalign_transform::validate::{ValidityLevel, validate_output, validate_to_level};
use tracing::{info, warn};

use crate::api::DurationMs;
use crate::chat_ops::fa::Grouping;
use crate::error::ServerError;
use crate::runner::util::{FileStage, ProgressSender, ProgressUpdate};
use crate::types::results::{FaGroupEvidence, FaResult};
use crate::types::traces::{FaEvidenceSourceTrace, FaGroupTrace, TimingTrace, ViolationTrace};
use transport::{FaInferencePlan, FaWorkerTransport, UncheckedFaWorkerBatch, plan_fa_inference};

/// Cache task name for FA results.
const CACHE_TASK: CacheTaskName = CacheTaskName::ForcedAlignment;
/// Cache namespace for immutable worker responses before local reconciliation.
const RAW_EVIDENCE_CACHE_TASK: CacheTaskName = CacheTaskName::ForcedAlignmentRawEvidence;

pub(super) fn collect_final_timings(
    all_timings: Vec<Option<Vec<Option<WordTiming>>>>,
    context: &str,
) -> Result<Vec<Vec<Option<WordTiming>>>, ServerError> {
    let missing_groups: Vec<usize> = all_timings
        .iter()
        .enumerate()
        .filter_map(|(index, timings)| timings.is_none().then_some(index))
        .collect();
    if !missing_groups.is_empty() {
        return Err(ServerError::Validation(format!(
            "{context} completed without timings for group(s): {missing_groups:?}"
        )));
    }

    // Safety: the None check above returned Err for any missing groups,
    // so all remaining elements are guaranteed Some.
    Ok(all_timings.into_iter().flatten().collect())
}

pub(super) fn collect_evidence_sources(
    sources: Vec<Option<FaEvidenceSourceTrace>>,
    context: &str,
) -> Result<Vec<FaEvidenceSourceTrace>, ServerError> {
    let missing_groups: Vec<usize> = sources
        .iter()
        .enumerate()
        .filter_map(|(index, source)| source.is_none().then_some(index))
        .collect();
    if !missing_groups.is_empty() {
        return Err(ServerError::Validation(format!(
            "{context} completed without an evidence source for group(s): {missing_groups:?}"
        )));
    }
    Ok(sources.into_iter().flatten().collect())
}

/// Cached timings proven to correspond one-to-one with a current FA group.
///
/// Deserialization establishes only the element type. This admission state
/// also proves the parallel-vector invariant before cached data can be labelled
/// as reusable evidence.
#[derive(Debug)]
pub(super) struct AdmittedCachedFaTimings(Vec<Option<WordTiming>>);

impl AdmittedCachedFaTimings {
    pub(super) fn decode(value: serde_json::Value, expected_words: usize) -> Result<Self, String> {
        let timings = serde_json::from_value::<Vec<Option<WordTiming>>>(value)
            .map_err(|error| error.to_string())?;
        if timings.len() != expected_words {
            return Err(format!(
                "cache entry contains {} cached timings for {expected_words} words",
                timings.len()
            ));
        }
        Ok(Self(timings))
    }

    pub(super) fn into_timings(self) -> Vec<Option<WordTiming>> {
        self.0
    }
}

/// Re-admit and locally reparse immutable FA worker evidence for one group.
fn replay_cached_raw_evidence(
    value: serde_json::Value,
    cache_key: &CacheKey,
    engine: crate::types::engines::FaEngineName,
    group_index: usize,
    group: &crate::chat_ops::fa::FaGroup,
    recording: &crate::chat_ops::fa::coordinates::Recording,
) -> Result<transport::FaWorkerEvidenceResult, ServerError> {
    let evidence = raw_evidence::FaRawEvidence::decode(
        value,
        engine,
        raw_evidence::ExpectedFaWords::new(group.words.len()),
        cache_key,
    )
    .map_err(|error| {
        ServerError::Validation(format!(
            "cached raw FA evidence for group {group_index} was refused: {error}"
        ))
    })?;
    transport::replay_group_evidence(evidence, group_index, group, recording)
}

/// A cache layer whose value was present but could not be admitted.
#[derive(Debug)]
struct RefusedFaCacheLayer {
    layer: &'static str,
    error: String,
}

/// The admitted cache state for one current FA group.
///
/// Raw worker evidence is intentionally tried first. Replaying it through the
/// current Rust projection is what lets alignment-algorithm experiments reuse
/// model inference. The derived timing layer remains a compatibility and
/// resilience fallback when raw evidence is absent or corrupt.
#[derive(Debug)]
enum AdmittedFaCacheGroup {
    RawEvidence(Box<transport::FaWorkerEvidenceResult>),
    DerivedTimings(AdmittedCachedFaTimings),
    Miss,
}

/// Result of checking both cache layers for one current FA group.
#[derive(Debug)]
struct FaCacheResolution {
    admitted: AdmittedFaCacheGroup,
    refusals: Vec<RefusedFaCacheLayer>,
}

fn resolve_cached_fa_group(
    raw_value: Option<&serde_json::Value>,
    derived_value: Option<&serde_json::Value>,
    cache_key: &CacheKey,
    engine: crate::types::engines::FaEngineName,
    group_index: usize,
    group: &crate::chat_ops::fa::FaGroup,
    recording: &crate::chat_ops::fa::coordinates::Recording,
) -> FaCacheResolution {
    let mut refusals = Vec::new();

    if let Some(value) = raw_value {
        match replay_cached_raw_evidence(
            value.clone(),
            cache_key,
            engine,
            group_index,
            group,
            recording,
        ) {
            Ok(evidence) => {
                return FaCacheResolution {
                    admitted: AdmittedFaCacheGroup::RawEvidence(Box::new(evidence)),
                    refusals,
                };
            }
            Err(error) => refusals.push(RefusedFaCacheLayer {
                layer: RAW_EVIDENCE_CACHE_TASK.as_str(),
                error: error.to_string(),
            }),
        }
    }

    if let Some(value) = derived_value {
        match AdmittedCachedFaTimings::decode(value.clone(), group.words.len()) {
            Ok(timings) => {
                return FaCacheResolution {
                    admitted: AdmittedFaCacheGroup::DerivedTimings(timings),
                    refusals,
                };
            }
            Err(error) => refusals.push(RefusedFaCacheLayer {
                layer: CACHE_TASK.as_str(),
                error,
            }),
        }
    }

    FaCacheResolution {
        admitted: AdmittedFaCacheGroup::Miss,
        refusals,
    }
}

/// Close the temporary indexed algorithm state into evidence that cannot
/// mispair one group's provenance, cache identity, or worker timing.
fn assemble_group_evidence(
    groups: Vec<FaGroupTrace>,
    evidence_sources: Vec<FaEvidenceSourceTrace>,
    cache_keys: Vec<String>,
    pre_injection_timings: Vec<Vec<Option<TimingTrace>>>,
) -> Result<Vec<FaGroupEvidence>, ServerError> {
    let lengths = [
        groups.len(),
        evidence_sources.len(),
        cache_keys.len(),
        pre_injection_timings.len(),
    ];
    if lengths.iter().any(|length| *length != lengths[0]) {
        return Err(ServerError::Validation(format!(
            "FA evidence cardinality drift: groups={}, sources={}, keys={}, timings={}",
            lengths[0], lengths[1], lengths[2], lengths[3]
        )));
    }
    Ok(groups
        .into_iter()
        .zip(evidence_sources)
        .zip(cache_keys)
        .zip(pre_injection_timings)
        .map(
            |(((group, source), cache_key), pre_injection_timings)| FaGroupEvidence {
                group,
                source,
                cache_key,
                pre_injection_timings,
            },
        )
        .collect())
}

// ---------------------------------------------------------------------------
// Per-file FA processing
// ---------------------------------------------------------------------------

/// Process a single CHAT file through the forced alignment pipeline.
///
/// Returns a structured [`FaResult`] containing the serialized CHAT text,
/// group info, timing data, and validation results.  The caller decides
/// which parts to persist (file output, trace cache, etc.).
///
/// Algorithm outline:
/// 1. Parse leniently and run pre-validation (`MainTierValid`).
/// 2. Group utterances into FA windows.
/// 3. Resolve cache hits/misses per group.
/// 4. Send miss groups through the FA worker transport adapter.
/// 5. Parse responses and align to transcript words in Rust.
/// 6. Apply timings + postprocessing (`apply_fa_results`).
/// 7. Run full post-validation and serialize.
pub(crate) async fn process_fa(
    chat_text: &str,
    audio: &AudioContext<'_>,
    worker_lang: &crate::api::LanguageCode3,
    services: PipelineServices<'_>,
    fa_params: &FaParams,
    progress: Option<&ProgressSender>,
) -> Result<FaResult, ServerError> {
    run_fa_impl(chat_text, audio, worker_lang, services, fa_params, progress).await
}

pub(crate) async fn run_fa_impl(
    chat_text: &str,
    audio: &AudioContext<'_>,
    worker_lang: &crate::api::LanguageCode3,
    services: PipelineServices<'_>,
    fa_params: &FaParams,
    progress: Option<&ProgressSender>,
) -> Result<FaResult, ServerError> {
    // 1. Parse
    let parser = crate::chat_parser();
    let (chat_file, parse_errors) = parse_lenient(&parser, chat_text);
    if !parse_errors.is_empty() {
        warn!(
            num_errors = parse_errors.len(),
            "Parse errors in FA input (continuing with recovery)"
        );
    }

    run_fa_from_ast(
        chat_file,
        parse_errors,
        audio,
        worker_lang,
        services,
        fa_params,
        progress,
    )
    .await
}

/// Run forced alignment on a pre-parsed `ChatFile`.
///
/// This is the primary FA entry point when the caller already owns a `ChatFile`
/// AST (e.g., after UTR injection). It avoids the serialize→re-parse cycle that
/// `process_fa(&str)` performs.
pub(crate) async fn run_fa_from_ast(
    mut chat_file: crate::chat_ops::ChatFile,
    parse_errors: Vec<crate::chat_ops::ParseError>,
    audio: &AudioContext<'_>,
    worker_lang: &crate::api::LanguageCode3,
    services: PipelineServices<'_>,
    fa_params: &FaParams,
    progress: Option<&ProgressSender>,
) -> Result<FaResult, ServerError> {
    // 1a′. Suppress %wor for Conversation Analysis transcripts.
    // CA transcripts (@Options: CA) use prosodic notation (⌈⌉⌊⌋, arrows,
    // lengthening marks) that %wor cannot represent. Generating %wor for
    // these files adds noise that CA researchers must manually remove.
    let write_wor = if is_ca(&chat_file) {
        info!("@Options: CA detected: suppressing %wor generation");
        false
    } else {
        fa_params.wor_tier.should_write()
    };

    // 1b. Skip dummy files
    if is_dummy(&chat_file) {
        return Ok(FaResult::without_groups(
            to_chat_string(&chat_file),
            fa_params.gap_healing,
            fa_params.engine.as_wire_name(),
            services.engine_version.as_ref(),
        ));
    }

    // 1c. @Options: NoAlign: strict pass-through, zero modifications.
    //
    // A researcher who sets this option has opted the file out of all
    // alignment processing.  The file is returned EXACTLY as parsed:
    // no timestamps added, removed, or adjusted, no %wor generated,
    // no decision tiers written.  This includes cleanup passes that
    // might seem safe (e.g., monotonicity enforcement); those are
    // the researcher's responsibility.
    //
    // See book/src/batchalign/developer/commands/align.md: "NoAlign: strict pass-through".
    if is_no_align(&chat_file) {
        return Ok(FaResult::without_groups(
            to_chat_string(&chat_file),
            fa_params.gap_healing,
            fa_params.engine.as_wire_name(),
            services.engine_version.as_ref(),
        ));
    }

    // 1d. Pre-validation gate (L2: MainTierValid)
    if let Err(errors) = validate_to_level(&chat_file, &parse_errors, ValidityLevel::MainTierValid)
    {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        return Err(ServerError::Validation(format!(
            "align pre-validation failed: {}",
            msgs.join("; ")
        )));
    }

    // 1e. Cheap rerun path: if the file already has complete, reusable `%wor`
    // timing, rebuild main-tier bullets and optionally regenerate `%wor`
    // without sending audio back through FA.
    if has_reusable_wor_timing(&chat_file) {
        info!("FA fast path: reusing existing %wor timing");
        refresh_existing_alignment(&mut chat_file, write_wor);

        // A previous run may have written backward `%wor` timestamps (e.g.
        // APROCSA 2256_T4.cha: UTR anchor drift placed utterances from task N
        // into task N-1's audio window).  Without these two steps, every re-run
        // reconstructs the backward main-tier bullet from the stale `%wor`
        // data, and the E362 violation persists indefinitely.
        //
        // Step 1: strip backward main-tier bullets.
        let decisions = enforce_monotonicity(&mut chat_file);
        // Step 2: remove `%wor` from stripped utterances so the next run goes
        // through full FA rather than reconstructing the backward bullet again.
        strip_wor_from_monotonicity_stripped_utterances(&mut chat_file, &decisions);

        let written = crate::chat_ops::fa::retain_decision_evidence(
            &mut chat_file,
            crate::chat_ops::fa::FaDecisions::without_injection(Vec::new(), Vec::new(), decisions),
        );

        return Ok(FaResult::without_groups(
            to_chat_string(&chat_file),
            fa_params.gap_healing,
            fa_params.engine.as_wire_name(),
            services.engine_version.as_ref(),
        )
        .with_written_decisions(written));
    }

    // 1f. Per-utterance partial reuse: when some (but not all) utterances have
    // clean %wor, refresh those and track them so their FA groups can be skipped.
    let reusable_indices = find_reusable_utterance_indices(&chat_file);
    if !reusable_indices.is_empty() {
        info!(
            reusable = reusable_indices.len(),
            "FA partial reuse: refreshing utterances with clean %wor"
        );
        refresh_reusable_utterances(&mut chat_file, &reusable_indices, write_wor);
    }

    // 2a. Rescue catastrophically narrow utterance bullets before grouping.
    //
    // When `transcribe` writes a bullet that is physically too narrow to
    // contain its words (e.g., 22 words in 380 ms = 58 wps, impossible),
    // FA cannot align the words against that audio range. Wave2Vec rejects
    // the group with "targets length is too long for CTC" because the
    // encoder produces too few frames for the target labels, and the
    // Whisper FA fallback path produces degenerate token-level timings
    // (zero-duration words, words past the bullet end). The user sees a
    // CHAT file with a `%wor` tier full of broken timings.
    //
    // The rescue pre-pass detects under-budgeted bullets and expands them
    // into the trailing inter-utterance gap, giving FA a wide-enough audio
    // window to find the actual speech. After FA finishes,
    // `update_utterance_bullet` overwrites the rescued range with the FA
    // word span (which is tighter), so the rescue is self-healing.
    //
    // Covered by the private regression fixture set under
    // `test-fixtures/align/regressions/` (gitignored; see
    // `book/src/batchalign/developer/regression-fixtures.md`).
    let rescue_decisions = rescue_narrow_bullets(&mut chat_file);

    // 2b. Expand utterance bullets to cover edge fillers in inter-utterance gaps.
    // UTR-assigned bullets may be too narrow to include trailing/leading fillers
    // whose audio lives in the gap between utterances.
    expand_bullets_for_edge_fillers(&mut chat_file);

    // Resolved once, here, and used for BOTH grouping and the containment
    // checks on what the engine returns. Grouping used to take an
    // `Option<u64>` and invent its own behaviour when it was absent; there is
    // one recording and one answer.
    let recording = audio.recording().await?;
    // 2c. Group utterances
    let Grouping {
        groups,
        refusals: unplaceable_decisions,
        windows_clamped,
    } = group_utterances(&chat_file, fa_params.max_group_ms().0, &recording);

    if groups.is_empty() {
        let monotonicity = enforce_monotonicity(&mut chat_file);
        strip_wor_from_monotonicity_stripped_utterances(&mut chat_file, &monotonicity);
        let written = crate::chat_ops::fa::retain_decision_evidence(
            &mut chat_file,
            crate::chat_ops::fa::FaDecisions::without_injection(
                rescue_decisions,
                unplaceable_decisions,
                monotonicity,
            ),
        );
        return Ok(FaResult::without_groups(
            to_chat_string(&chat_file),
            fa_params.gap_healing,
            fa_params.engine.as_wire_name(),
            services.engine_version.as_ref(),
        )
        .with_written_decisions(written));
    }

    info!(
        num_groups = groups.len(),
        total_words = groups.iter().map(|g| g.words.len()).sum::<usize>(),
        // Reported beside the other grouping facts rather than only as its own
        // warning, so a reader of one line sees whether our gap arithmetic
        // overshot the audio.
        windows_clamped,
        "FA grouping complete"
    );

    if let Some(tx) = progress {
        let _ = tx.send(ProgressUpdate::new(
            FileStage::CheckingCache,
            Some(0),
            Some(groups.len() as i64),
        ));
    }

    // 3. For each group: compute cache key, check cache
    let word_texts: Vec<Vec<String>> = groups
        .iter()
        .map(|g| g.words.iter().map(|w| w.text.clone()).collect())
        .collect();

    let cache_keys: Vec<CacheKey> = groups
        .iter()
        .zip(word_texts.iter())
        .map(|(g, words)| {
            cache_key(
                words,
                audio.audio_identity,
                g.audio_start_ms(),
                g.audio_end_ms(),
                fa_params.gap_healing,
                fa_params.engine,
            )
        })
        .collect();

    // 4. Cache lookup
    let key_strings: Vec<String> = cache_keys.iter().map(|k| k.as_str().to_string()).collect();
    let cached = match fa_params.cache_policy {
        crate::params::CachePolicy::SkipCache => std::collections::HashMap::new(),
        crate::params::CachePolicy::UseCache | crate::params::CachePolicy::RequireCache => {
            match services
                .cache
                .get_batch(&key_strings, CACHE_TASK.as_str(), services.engine_version)
                .await
            {
                Ok(map) => map,
                Err(e) => {
                    warn!(error = %e, "FA cache batch lookup failed (treating all as misses)");
                    std::collections::HashMap::new()
                }
            }
        }
    };
    let cached_raw = match fa_params.cache_policy {
        crate::params::CachePolicy::SkipCache => std::collections::HashMap::new(),
        crate::params::CachePolicy::UseCache | crate::params::CachePolicy::RequireCache => {
            match services
                .cache
                .get_batch(
                    &key_strings,
                    RAW_EVIDENCE_CACHE_TASK.as_str(),
                    services.engine_version,
                )
                .await
            {
                Ok(map) => map,
                Err(error) => {
                    warn!(error = %error, "Raw FA evidence cache batch lookup failed");
                    std::collections::HashMap::new()
                }
            }
        }
    };

    // 5. Partition into reused (from %wor), cache hits, and misses
    let mut all_timings: Vec<Option<Vec<Option<WordTiming>>>> = vec![None; groups.len()];
    let mut evidence_sources: Vec<Option<FaEvidenceSourceTrace>> = vec![None; groups.len()];
    let mut miss_indices: Vec<usize> = Vec::new();
    let mut reused_group_count = 0usize;
    let mut fallback_events = Vec::new();

    for (i, key) in cache_keys.iter().enumerate() {
        // Tier 1: group fully reusable from %wor (all utterances have clean timing)
        if !reusable_indices.is_empty()
            && groups[i]
                .utterance_indices
                .iter()
                .all(|idx| reusable_indices.contains(&idx.raw()))
            && let Some(timings) =
                incremental::collect_preserved_group_timings(&chat_file, &groups[i])
        {
            all_timings[i] = Some(timings);
            evidence_sources[i] = Some(FaEvidenceSourceTrace::WorReuse);
            reused_group_count += 1;
            continue;
        }

        // Tier 2: replay immutable worker evidence through current Rust logic.
        // Tier 3: fall back to the admitted derived timing cache when raw
        // evidence is unavailable. This ordering is what makes local algorithm
        // experiments inference-free.
        let resolution = resolve_cached_fa_group(
            cached_raw.get(key.as_str()),
            cached.get(key.as_str()),
            key,
            fa_params.engine,
            i,
            &groups[i],
            &recording,
        );
        for refusal in resolution.refusals {
            warn!(
                error = %refusal.error,
                cache_layer = refusal.layer,
                group = i,
                "Cached FA evidence was refused"
            );
        }
        match resolution.admitted {
            AdmittedFaCacheGroup::RawEvidence(evidence) => {
                let evidence = *evidence;
                if let Some(event) = evidence.fallback_event {
                    fallback_events.push(event);
                }
                all_timings[i] = Some(evidence.timings);
                evidence_sources[i] = Some(FaEvidenceSourceTrace::RawEvidenceReplay);
            }
            AdmittedFaCacheGroup::DerivedTimings(timings) => {
                all_timings[i] = Some(timings.into_timings());
                evidence_sources[i] = Some(FaEvidenceSourceTrace::Cache);
            }
            AdmittedFaCacheGroup::Miss => {
                // Tier 4: no admitted cache evidence, so inference is needed.
                miss_indices.push(i);
            }
        }
    }

    let cache_hits = groups.len() - miss_indices.len() - reused_group_count;
    let reused_or_cached_groups = reused_group_count + cache_hits;
    if cache_hits > 0 || reused_group_count > 0 {
        info!(
            reused = reused_group_count,
            cache_hits = cache_hits,
            misses = miss_indices.len(),
            "FA partition (reused from %wor / cache hits / misses)"
        );
    }

    if let Some(tx) = progress {
        let _ = tx.send(ProgressUpdate::new(
            FileStage::Aligning,
            Some(reused_or_cached_groups as i64),
            Some(groups.len() as i64),
        ));
    }

    let transport = FaWorkerTransport::production(services);

    // 6. Dispatch miss groups through the FA worker transport adapter
    if let FaInferencePlan::Authorized(authorization) =
        plan_fa_inference(fa_params.cache_policy, &miss_indices)?
    {
        // Resolved before dispatch so every group's reply can be checked
        // against the audio it describes. Fails the file rather than running
        // unbounded: a pass that cannot state the recording's length cannot
        // tell a measurement from a moment that does not exist.
        let parsed_results = transport
            .infer_groups(
                UncheckedFaWorkerBatch {
                    word_texts: &word_texts,
                    groups: &groups,
                    cache_keys: &cache_keys,
                    authorization,
                    audio_path: audio.audio_path,
                    worker_lang: worker_lang.into(),
                    engine: fa_params.engine,
                    gap_healing: fa_params.gap_healing,
                    recording,
                }
                .admit()?,
            )
            .await?;

        for (parsed_idx, parsed_result) in parsed_results.into_iter().enumerate() {
            let (miss_idx, timings, raw_evidence, fallback_event) = match parsed_result {
                transport::FaWorkerGroupResult::Evidence(evidence) => {
                    let evidence = *evidence;
                    (
                        evidence.group_index,
                        evidence.timings,
                        Some(evidence.raw_evidence),
                        evidence.fallback_event,
                    )
                }
                transport::FaWorkerGroupResult::Unaligned(unaligned) => (
                    unaligned.group_index,
                    vec![None; unaligned.word_count],
                    None,
                    None,
                ),
            };
            if let Some(event) = fallback_event {
                fallback_events.push(event);
            }

            // Cache the result
            let ba_version = env!("CARGO_PKG_VERSION");
            if let Ok(cache_data) = serde_json::to_value(&timings)
                && let Err(e) = services
                    .cache
                    .put_batch(
                        &[(cache_keys[miss_idx].as_str().to_string(), cache_data)],
                        CACHE_TASK.as_str(),
                        services.engine_version,
                        ba_version,
                    )
                    .await
            {
                warn!(error = %e, "Failed to cache FA result (non-fatal)");
            }
            if let Some(raw_evidence) = raw_evidence {
                match serde_json::to_value(raw_evidence) {
                    Ok(cache_data) => {
                        if let Err(error) = services
                            .cache
                            .put_batch(
                                &[(cache_keys[miss_idx].as_str().to_string(), cache_data)],
                                RAW_EVIDENCE_CACHE_TASK.as_str(),
                                services.engine_version,
                                ba_version,
                            )
                            .await
                        {
                            warn!(error = %error, "Failed to cache raw FA evidence (non-fatal)");
                        }
                    }
                    Err(error) => {
                        warn!(error = %error, "Failed to serialize raw FA evidence (non-fatal)");
                    }
                }
            }
            all_timings[miss_idx] = Some(timings);
            evidence_sources[miss_idx] = Some(FaEvidenceSourceTrace::Inference);

            if let Some(tx) = progress {
                let done = reused_or_cached_groups + parsed_idx + 1;
                let _ = tx.send(ProgressUpdate::new(
                    FileStage::Aligning,
                    Some(done as i64),
                    Some(groups.len() as i64),
                ));
            }
        }
    }

    // 8. Apply all results
    if let Some(tx) = progress {
        let _ = tx.send(ProgressUpdate::new(
            FileStage::ApplyingResults,
            Some(groups.len() as i64),
            Some(groups.len() as i64),
        ));
    }

    let final_timings = collect_final_timings(all_timings, "forced alignment")?;
    let evidence_sources = collect_evidence_sources(evidence_sources, "forced alignment")?;

    // Snapshot pre-injection timings (before apply_fa_results consumes them)
    let pre_injection_timings: Vec<Vec<Option<TimingTrace>>> = final_timings
        .iter()
        .map(|group| {
            group
                .iter()
                .map(|t| t.as_ref().map(TimingTrace::from_word_timing))
                .collect()
        })
        .collect();

    let fa_applied = apply_fa_results(
        &mut chat_file,
        &groups,
        &final_timings,
        fa_params.word_end_policy(),
        write_wor,
    );

    // 9. Post-FA bullet repair (experimental, opt-in via --bullet-repair).
    let repair_decisions = if fa_params.bullet_repair {
        let repair_result = crate::chat_ops::fa::repair_bullets(&mut chat_file, false);
        tracing::info!(%repair_result.stats, "bullet repair applied");
        repair_result.decisions
    } else {
        Vec::new()
    };

    // 9b. (Decision tiers injected in step 9d below, after all decisions are collected.)

    // 9c. Enforce monotonicity: strip non-monotonic start times and clamp
    //    end-time overlaps. The old enforcement was removed (see comment in
    //    apply_fa_results) because it stripped too aggressively. The current
    //    version strips every start-time regression and clamps end times to
    //    the next utterance's start. Timing removal is now retained as a typed
    //    decision in both the optional CHAT projection and durable evidence.
    let ordered = fa_applied.then_enforce_monotonicity(&mut chat_file);

    // 9d. Retain decision provenance for all pipeline decisions that altered
    //    the output and strip abandoned review tiers. Ordering and retention
    //    live in `retain_decision_evidence`; this states the SOURCES only, and
    //    a sixth would not compile until both FA paths named it.
    let written_decisions = crate::chat_ops::fa::retain_decision_evidence(
        &mut chat_file,
        crate::chat_ops::fa::FaDecisions {
            rescue: rescue_decisions,
            unplaceable: unplaceable_decisions,
            ordered,
            repair: repair_decisions.iter().map(Into::into).collect(),
        },
    );
    let (decision_records, timing_effects) = written_decisions.into_evidence();
    let decision_traces = decision_records.into_iter().map(Into::into).collect();
    let timing_decisions = timing_effects.into_iter().map(Into::into).collect();

    // 10. Post-validation check (warn only, cross-speaker overlap is normal in
    //    conversation data).
    let violations = if let Err(errors) = validate_output(&chat_file, "align") {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        warn!(errors = ?msgs, "align post-validation warnings (non-fatal)");
        errors
            .iter()
            .map(|e| ViolationTrace {
                code: format!("L{}", e.level as u8),
                message: e.message.clone(),
                utterance_index: None,
            })
            .collect()
    } else {
        Vec::new()
    };

    // 10. Build group traces
    let group_traces: Vec<FaGroupTrace> = groups
        .iter()
        .map(|g| FaGroupTrace {
            audio_start_ms: DurationMs(g.audio_start_ms()),
            audio_end_ms: DurationMs(g.audio_end_ms()),
            utterance_indices: g.utterance_indices.iter().map(|idx| idx.raw()).collect(),
            words: g.words.iter().map(|w| w.text.clone()).collect(),
            word_ids: g.words.iter().map(|word| word.stable_id()).collect(),
        })
        .collect();

    // 11. Serialize and return structured result
    let group_evidence = assemble_group_evidence(
        group_traces,
        evidence_sources,
        cache_keys
            .iter()
            .map(|key| key.as_str().to_owned())
            .collect(),
        pre_injection_timings,
    )?;

    Ok(FaResult {
        chat_text: to_chat_string(&chat_file),
        group_evidence,
        engine: fa_params.engine.as_wire_name().to_owned(),
        engine_version: services.engine_version.as_ref().to_owned(),
        decisions: decision_traces,
        timing_decisions,
        gap_healing: fa_params.gap_healing,
        violations,
        fallback_events,
    })
}

mod incremental;
pub(crate) use incremental::process_fa_incremental;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_task_name_is_stable() {
        assert_eq!(CACHE_TASK.as_str(), "forced_alignment");
        assert_eq!(
            RAW_EVIDENCE_CACHE_TASK.as_str(),
            "forced_alignment_raw_evidence"
        );
    }

    #[test]
    fn collect_final_timings_rejects_missing_groups() {
        let error = collect_final_timings(vec![Some(Vec::new()), None], "forced alignment")
            .expect_err("missing timing groups should fail");
        assert!(
            error
                .to_string()
                .contains("completed without timings for group(s): [1]")
        );
    }

    #[test]
    fn collect_evidence_sources_rejects_missing_groups() {
        let error = collect_evidence_sources(
            vec![Some(FaEvidenceSourceTrace::Cache), None],
            "forced alignment",
        )
        .expect_err("missing evidence-source groups should fail");
        assert!(
            error
                .to_string()
                .contains("completed without an evidence source for group(s): [1]")
        );
    }

    #[test]
    fn group_evidence_assembly_rejects_parallel_cardinality_drift() {
        let error = assemble_group_evidence(
            Vec::new(),
            vec![FaEvidenceSourceTrace::Cache],
            Vec::new(),
            Vec::new(),
        )
        .expect_err("parallel FA evidence with different lengths must fail");

        assert!(error.to_string().contains("FA evidence cardinality drift"));
    }

    #[test]
    fn cached_group_timing_admission_refuses_word_count_drift() {
        let value = serde_json::json!([null]);

        let error = AdmittedCachedFaTimings::decode(value, 2)
            .expect_err("a short cache vector must not become group timing evidence");

        assert!(error.contains("1 cached timings for 2 words"));
    }

    #[test]
    fn cached_group_timing_admission_accepts_exact_parallel_shape() {
        let value = serde_json::json!([null, null]);

        let timings = AdmittedCachedFaTimings::decode(value, 2)
            .expect("an exact cache vector should be admitted")
            .into_timings();

        assert_eq!(timings, vec![None, None]);
    }

    #[test]
    fn raw_evidence_is_replayed_before_a_derived_timing_hit() {
        use crate::api::DurationSeconds;
        use crate::chat_ops::fa::coordinates::{Ms, Recording};
        use crate::chat_ops::fa::{FaGroup, FaWord, TimeSpan};
        use crate::chat_ops::{UtteranceIdx, WordIdx};
        use crate::types::engines::FaEngineName;
        use crate::types::worker_v2::{
            ExecuteResponseV2, IndexedWordTimingResultV2, TaskResultV2, WorkerRequestIdV2,
        };

        let key = CacheKey::from_content("raw-first");
        let group = FaGroup {
            audio_span: TimeSpan::new(100, 900),
            words: vec![FaWord {
                utterance_index: UtteranceIdx::new(0),
                utterance_word_index: WordIdx::new(0),
                text: "hello".to_owned(),
            }],
            utterance_indices: vec![UtteranceIdx::new(0)],
        };
        let response = ExecuteResponseV2::success(
            WorkerRequestIdV2::from("raw-first"),
            TaskResultV2::IndexedWordTimingResult(IndexedWordTimingResultV2 {
                indexed_timings: vec![None],
            }),
            DurationSeconds(0.01),
        );
        let raw = raw_evidence::FaRawEvidence::admit_requested(
            &response,
            FaEngineName::Wave2Vec,
            raw_evidence::ExpectedFaWords::new(1),
            &key,
            raw_evidence::FaEvidenceRoute::Direct,
        )
        .expect("fixture evidence is valid");
        let raw_json = serde_json::to_value(raw).expect("serialize raw evidence");
        let derived_timing = WordTiming::new(
            110,
            180,
            crate::chat_ops::fa::origin::Origin::TranscriptBullet,
            crate::chat_ops::fa::origin::Origin::TranscriptBullet,
        )
        .expect("positive derived timing");
        let derived_json =
            serde_json::to_value(vec![Some(derived_timing)]).expect("serialize derived timing");
        let recording = Recording::of_duration(Ms(1_000)).expect("non-empty recording");

        let resolution = resolve_cached_fa_group(
            Some(&raw_json),
            Some(&derived_json),
            &key,
            FaEngineName::Wave2Vec,
            0,
            &group,
            &recording,
        );

        assert!(resolution.refusals.is_empty());
        match resolution.admitted {
            AdmittedFaCacheGroup::RawEvidence(evidence) => {
                assert_eq!(evidence.timings, vec![None]);
            }
            other => panic!("raw evidence must win over derived timings, got {other:?}"),
        }
    }
}
