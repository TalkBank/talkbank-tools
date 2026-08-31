//! Typed dispatch plans derived from persisted runner snapshots.
//!
//! The store owns durable job state (`RunnerJobSnapshot`, `CommandOptions`,
//! `runtime_state`). Dispatch modules own orchestration. This module is the seam
//! between those responsibilities: it translates the store-facing shapes once
//! into narrower, command-family-specific plans before orchestration begins.

use crate::chat_ops::morphosyntax_ops::{MultilingualPolicy, MwtDict, TokenizationMode};

use crate::api::ReleasedCommand;
use crate::chat_ops::CacheTaskName;
use crate::config::ServerConfig;
use crate::host_policy::HostExecutionPolicy;
use crate::params::{CacheOverrides, CachePolicy};
use crate::runner::dispatch::kernel_plan::CommandKernelPlan;
use crate::store::RunnerJobSnapshot;
use crate::transcribe::{AsrBackend, TranscribeCachePolicies, TranscribeOptions};
use crate::types::worker_v2::SpeakerBackendV2;

use super::options::{
    BenchmarkDispatchParams, FaDispatchParams, MorphotagDispatchParams, OpensmileDispatchParams,
    TranscribeDispatchParams, extract_benchmark_dispatch_params, extract_fa_dispatch_params,
    extract_morphotag_dispatch_params, extract_opensmile_dispatch_params,
    extract_transcribe_dispatch_params,
};

/// Typed plan for the batched text infer family.
///
/// This plan carries the option-derived behavior knobs for the
/// morphotag / utseg / translate / coref / compare commands. Its consumer is
/// `runner::routing`, which reads these knobs to build the runtime options for
/// the recipe-owned execution path in `crate::execution`. It no longer feeds a
/// dispatch module of its own: the batched-text dispatch module was retired
/// once every one of those five commands had a name-matched arm.
///
/// It deliberately carries NO resource-execution profile. It used to hold a
/// `CommandKernelPlan`, whose only reader was that retired dispatch module; the
/// recipe stack derives parallelism from the recipe instead. Do not re-add one
/// speculatively.
#[derive(Clone)]
pub(crate) struct BatchedInferDispatchPlan {
    /// Morphotag-specific retokenization policy. Other text commands keep the
    /// default `Preserve` behavior.
    pub tokenization_mode: TokenizationMode,
    /// Morphotag-specific multilingual routing policy.
    pub multilingual_policy: MultilingualPolicy,
    /// Whether output should pass through merge-abbrev before persistence.
    pub should_merge_abbrev: bool,
    /// Optional multi-word-token lexicon loaded by the CLI.
    pub mwt: MwtDict,
    /// [Experimental] Route @s words to secondary language Stanza models.
    pub l2_morphotag: bool,
    /// Apply transcriber `$POS` hints as a post-pass on %mor (default on;
    /// CLI exposes `--no-pos-hints` to opt out).
    pub respect_pos_hints: bool,
    /// Legacy review-level request retained for stored-job compatibility.
    /// No value emits CHAT decision tiers.
    pub review_level: crate::chat_ops::fa::ReviewLevel,
}

impl BatchedInferDispatchPlan {
    /// Build the batched-text plan once from the runner snapshot.
    ///
    /// Takes no `ServerConfig`: every field is derived from the job's submitted
    /// options alone. That became true when the plan's `CommandKernelPlan` went
    /// away with the retired batched-text dispatch, and it is worth preserving,
    /// since a plan that depends on nothing host-shaped is one that can move
    /// into a pure crate unchanged.
    pub(crate) fn from_job(job: &RunnerJobSnapshot) -> Self {
        let morphotag_params = extract_morphotag_dispatch_params(&job.dispatch.options);
        let MorphotagDispatchParams {
            tokenization_mode,
            multilingual_policy,
            override_media_cache: _,
            merge_abbrev,
            l2_morphotag,
            respect_pos_hints,
            review_level,
        } = morphotag_params.unwrap_or(MorphotagDispatchParams {
            tokenization_mode: TokenizationMode::Preserve,
            multilingual_policy: MultilingualPolicy::ProcessAll,
            override_media_cache: job.dispatch.options.common().override_media_cache,
            merge_abbrev: job.dispatch.options.merge_abbrev_policy(),
            l2_morphotag: false,
            respect_pos_hints: false,
            review_level: crate::chat_ops::fa::ReviewLevel::None,
        });

        Self {
            tokenization_mode,
            multilingual_policy,
            should_merge_abbrev: merge_abbrev.should_merge(),
            mwt: job.dispatch.options.common().mwt.clone(),
            l2_morphotag,
            respect_pos_hints,
            review_level,
        }
    }
}

/// Typed plan for forced alignment dispatch.
pub(crate) struct FaDispatchPlan {
    /// Resource-aware execution profile for the command's remaining workload.
    pub kernel_plan: CommandKernelPlan,
    /// Fully extracted FA option bundle.
    pub options: FaDispatchParams,
    /// Cache policy for the UTR ASR pre-pass and fallback paths.
    ///
    /// UTR and forced alignment are independently addressable cache tasks.
    /// Keeping this policy out of `FaParams` prevents an FA refresh request
    /// from changing UTR behavior, or vice versa.
    pub utr_cache_policy: CachePolicy,
}

impl FaDispatchPlan {
    /// Build the FA option plan from the persisted job snapshot.
    pub(crate) fn from_job(job: &RunnerJobSnapshot, config: &ServerConfig) -> Option<Self> {
        let overrides = resolve_cache_overrides(job);
        let fa_cache_policy = overrides.policy_for(CacheTaskName::ForcedAlignment);
        let utr_cache_policy = overrides.policy_for(CacheTaskName::UtrAsr);
        extract_fa_dispatch_params(&job.dispatch.options, fa_cache_policy).map(|options| Self {
            kernel_plan: kernel_plan_for_job(job, config),
            options,
            utr_cache_policy,
        })
    }
}

/// Typed plan for transcribe dispatch.
///
/// The transcribe pipeline consumes a concrete `TranscribeOptions` bundle plus
/// the write-side merge-abbrev decision. Runtime-only toggles (`utseg`,
/// `morphosyntax`) are resolved here so the dispatch module stops re-reading
/// the store-owned `runtime_state` bag.
#[derive(Clone)]
pub(crate) struct TranscribeDispatchPlan {
    /// Resource-aware execution profile for the command's remaining workload.
    pub kernel_plan: CommandKernelPlan,
    /// Base transcribe options cloned per file before media-specific values are
    /// filled in.
    pub base_options: TranscribeOptions,
    /// Whether output should pass through merge-abbrev before persistence.
    pub should_merge_abbrev: bool,
}

impl TranscribeDispatchPlan {
    /// Build the transcribe plan from the persisted job snapshot.
    pub(crate) fn from_job(job: &RunnerJobSnapshot, config: &ServerConfig) -> Option<Self> {
        let overrides = resolve_cache_overrides(job);
        let cache_policies = TranscribeCachePolicies {
            rev_asr: overrides.policy_for(CacheTaskName::RevAsrEvidence),
            speaker: overrides.policy_for(CacheTaskName::SpeakerDiarizationRawEvidence),
        };
        let TranscribeDispatchParams {
            asr_engine,
            speaker_engine,
            diarize,
            merge_abbrev,
            cache_policies,
            wor_tier,
            allow_stanza_fallback_utseg,
            batch_size: _,
            engine_extras,
        } = extract_transcribe_dispatch_params(&job.dispatch.options, cache_policies)?;
        let with_utseg = runtime_flag(job, "utseg", true);
        let with_morphosyntax = runtime_flag(job, "morphosyntax", false);
        let speaker_backend = diarize.then(|| resolve_speaker_backend(speaker_engine));

        Some(Self {
            kernel_plan: kernel_plan_for_job(job, config),
            base_options: TranscribeOptions {
                backend: AsrBackend::from_engine_name(asr_engine.as_wire_name()),
                diarize,
                speaker_backend,
                lang: job.dispatch.lang.clone(),
                num_speakers: job.dispatch.num_speakers.0 as usize,
                with_utseg,
                with_morphosyntax,
                cache_policies,
                allow_stanza_fallback_utseg,
                write_wor: wor_tier.should_write(),
                media_name: None,
                engine_extras,
            },
            should_merge_abbrev: merge_abbrev.should_merge(),
        })
    }
}

/// Typed plan for benchmark dispatch.
#[derive(Clone)]
pub(crate) struct BenchmarkDispatchPlan {
    /// Resource-aware execution profile for the command's remaining workload.
    pub kernel_plan: CommandKernelPlan,
    /// Base transcribe options reused by the benchmark pipeline's ASR phase.
    pub base_options: TranscribeOptions,
    /// MWT dictionary handed to the compare phase.
    pub mwt: MwtDict,
    /// Whether the hypothesis CHAT output should merge abbreviations.
    pub should_merge_abbrev: bool,
}

impl BenchmarkDispatchPlan {
    /// Build the benchmark plan from the persisted job snapshot.
    pub(crate) fn from_job(job: &RunnerJobSnapshot, config: &ServerConfig) -> Option<Self> {
        let cache_policy = resolve_cache_overrides(job).policy_for(CacheTaskName::RevAsrEvidence);
        let BenchmarkDispatchParams {
            asr_engine,
            wor_tier,
            merge_abbrev,
            cache_policy,
            engine_extras,
        } = extract_benchmark_dispatch_params(&job.dispatch.options, cache_policy)?;

        Some(Self {
            kernel_plan: kernel_plan_for_job(job, config),
            base_options: TranscribeOptions {
                backend: AsrBackend::from_engine_name(asr_engine.as_wire_name()),
                diarize: false,
                speaker_backend: None,
                lang: job.dispatch.lang.clone(),
                num_speakers: job.dispatch.num_speakers.0 as usize,
                with_utseg: false,
                with_morphosyntax: false,
                cache_policies: TranscribeCachePolicies::uniform(cache_policy),
                allow_stanza_fallback_utseg: false,
                write_wor: wor_tier.should_write(),
                media_name: None,
                engine_extras,
            },
            mwt: MwtDict::default(),
            should_merge_abbrev: merge_abbrev.should_merge(),
        })
    }
}

/// Typed plan for media-analysis dispatch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MediaAnalysisDispatchPlan {
    /// OpenSMILE needs the selected feature-set string.
    Opensmile {
        /// Resource-aware execution profile for the command's remaining workload.
        kernel_plan: CommandKernelPlan,
        /// Feature set to request from the worker.
        feature_set: String,
    },
    /// AVQI currently has no command-specific options.
    Avqi {
        /// Resource-aware execution profile for the command's remaining workload.
        kernel_plan: CommandKernelPlan,
    },
    /// Standalone speaker diarization to a turns JSON artifact.
    Diarize {
        /// Resource-aware execution profile for the command's remaining workload.
        kernel_plan: CommandKernelPlan,
        /// Expected speaker count; `None` lets the diarizer auto-detect.
        expected_speakers: Option<crate::api::NumSpeakers>,
    },
}

impl MediaAnalysisDispatchPlan {
    /// Build the media-analysis plan from the persisted job snapshot.
    pub(crate) fn from_job(job: &RunnerJobSnapshot, config: &ServerConfig) -> Option<Self> {
        match job.dispatch.command {
            ReleasedCommand::Opensmile => {
                let OpensmileDispatchParams { feature_set } =
                    extract_opensmile_dispatch_params(&job.dispatch.options)?;
                Some(Self::Opensmile {
                    kernel_plan: kernel_plan_for_job(job, config),
                    feature_set,
                })
            }
            ReleasedCommand::Avqi => Some(Self::Avqi {
                kernel_plan: kernel_plan_for_job(job, config),
            }),
            ReleasedCommand::Diarize => {
                // A diarize job snapshot must carry diarize options; any
                // other variant means the persisted options row is corrupt.
                // Returning `None` fails plan construction visibly (same
                // contract as the opensmile params extractor above) instead
                // of silently proceeding with default settings.
                let crate::options::CommandOptions::Diarize(options) = &job.dispatch.options else {
                    return None;
                };
                Some(Self::Diarize {
                    kernel_plan: kernel_plan_for_job(job, config),
                    expected_speakers: options.expected_speakers,
                })
            }
            _ => None,
        }
    }
}

/// Resolve [`CacheOverrides`] from the common options on a job snapshot.
///
/// Reads `override_media_cache_tasks` (per-task) and `override_media_cache` (all-or-nothing)
/// from `CommonOptions` and produces a typed `CacheOverrides` value.
fn resolve_cache_overrides(job: &RunnerJobSnapshot) -> CacheOverrides {
    let common = job.dispatch.options.common();
    if common.require_media_cache {
        CacheOverrides::RequireAll
    } else if !common.override_media_cache_tasks.is_empty() {
        let tasks = common
            .override_media_cache_tasks
            .iter()
            .filter_map(|s| parse_cache_task_name(s))
            .collect();
        CacheOverrides::Tasks(tasks)
    } else if common.override_media_cache {
        CacheOverrides::All
    } else {
        CacheOverrides::None
    }
}

fn kernel_plan_for_job(job: &RunnerJobSnapshot, config: &ServerConfig) -> CommandKernelPlan {
    let host_policy = HostExecutionPolicy::from_server_config(config);
    CommandKernelPlan::for_command_with_policy(
        job.dispatch.command,
        job.pending_files.len(),
        &host_policy,
    )
}

/// Parse a wire name into a [`CacheTaskName`].
///
/// Only audio tasks are cached, so text-task names resolve to `None`
/// with a single warning. Unrecognized names also resolve to `None`,
/// silently (CLI clap validation already rejects truly unknown input).
fn parse_cache_task_name(name: &str) -> Option<CacheTaskName> {
    use crate::chat_ops::cache_key::CacheOverrideTaskName;

    match CacheTaskName::classify_override_name(name) {
        CacheOverrideTaskName::Cacheable(task) => Some(task),
        CacheOverrideTaskName::TextNlpUnsupported => {
            tracing::warn!(
                task = name,
                "--override-media-cache-tasks ignored for text-NLP task (batchalign3 does not cache text NLP)"
            );
            None
        }
        CacheOverrideTaskName::Unknown => None,
    }
}

/// Resolve one runtime-only flag with its documented default.
fn runtime_flag(job: &RunnerJobSnapshot, key: &str, default: bool) -> bool {
    job.dispatch
        .runtime_state
        .get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or(default)
}

/// Resolve the dedicated speaker backend from `engine_overrides`.
fn resolve_speaker_backend(engine: Option<crate::options::SpeakerEngineName>) -> SpeakerBackendV2 {
    match engine.unwrap_or(crate::options::SpeakerEngineName::PyannoteAi) {
        crate::options::SpeakerEngineName::PyannoteAi => SpeakerBackendV2::PyannoteAi,
        crate::options::SpeakerEngineName::Pyannote => SpeakerBackendV2::Pyannote,
        crate::options::SpeakerEngineName::Nemo => SpeakerBackendV2::Nemo,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::api::{JobId, LanguageCode3, NumSpeakers, ReleasedCommand};
    use crate::config::ServerConfig;
    use crate::options::{
        AlignOptions, AsrEngineName, BenchmarkOptions, CommandOptions, CommonOptions,
        MorphotagOptions, OpensmileOptions, TranscribeOptions as TranscribeCommand,
    };
    use crate::store::{
        RunnerDispatchConfig, RunnerFilesystemConfig, RunnerJobIdentity, RunnerJobSnapshot,
    };
    use crate::transcribe::AsrWorkerMode;

    fn make_snapshot(
        command: ReleasedCommand,
        options: CommandOptions,
        runtime_state: BTreeMap<String, serde_json::Value>,
    ) -> RunnerJobSnapshot {
        RunnerJobSnapshot {
            run_generation: crate::store::RunGeneration::FIRST,
            identity: RunnerJobIdentity {
                job_id: JobId::from("job-plan"),
                correlation_id: "test-correlation".into(),
            },
            dispatch: RunnerDispatchConfig {
                command,
                lang: crate::api::LanguageSpec::Resolved(LanguageCode3::eng()),
                num_speakers: NumSpeakers(3),
                options,
                runtime_state,
                debug_traces: false,
            },
            filesystem: RunnerFilesystemConfig {
                paths_mode: false,
                source_paths: Vec::new(),
                output_paths: Vec::new(),
                before_paths: Vec::new(),
                staging_dir: Default::default(),
                media_mapping: Default::default(),
                media_subdir: Default::default(),
                source_dir: Default::default(),
            },
            cancel_token: CancellationToken::new(),
            pending_files: Vec::new(),
        }
    }

    #[test]
    fn batched_plan_uses_morphotag_translation() {
        let mut common = CommonOptions {
            override_media_cache: true,
            ..Default::default()
        };
        common
            .mwt
            .insert("gonna".into(), vec!["going".into(), "to".into()]);
        let snapshot = make_snapshot(
            ReleasedCommand::Morphotag,
            CommandOptions::Morphotag(MorphotagOptions {
                common,
                retokenize: true,
                skipmultilang: true,
                merge_abbrev: true.into(),

                ..Default::default()
            }),
            BTreeMap::new(),
        );

        let plan = BatchedInferDispatchPlan::from_job(&snapshot);

        assert_eq!(plan.tokenization_mode, TokenizationMode::StanzaRetokenize);
        assert_eq!(plan.multilingual_policy, MultilingualPolicy::SkipNonPrimary);
        assert!(plan.should_merge_abbrev);
        assert_eq!(
            plan.mwt.get("gonna"),
            Some(&vec!["going".to_string(), "to".to_string()])
        );
    }

    #[test]
    fn transcribe_plan_reads_runtime_flags_and_speaker_override() {
        let common = CommonOptions {
            override_media_cache: true,
            ..Default::default()
        };
        let mut runtime_state = BTreeMap::new();
        runtime_state.insert("utseg".into(), json!(false));
        runtime_state.insert("morphosyntax".into(), json!(true));
        let snapshot = make_snapshot(
            ReleasedCommand::Transcribe,
            CommandOptions::Transcribe(TranscribeCommand {
                common,
                asr_engine: AsrEngineName::HkAliyun,
                diarize: true,
                wor: false.into(),
                merge_abbrev: true.into(),
                batch_size: 32,
                utseg_fallback: false.into(),
            }),
            runtime_state,
        );

        let plan = TranscribeDispatchPlan::from_job(&snapshot, &ServerConfig::default())
            .expect("transcribe plan");

        assert!(matches!(
            plan.base_options.backend,
            AsrBackend::Worker(AsrWorkerMode::HkAliyunV2)
        ));
        assert!(plan.base_options.diarize);
        assert_eq!(
            plan.base_options.speaker_backend,
            Some(SpeakerBackendV2::PyannoteAi)
        );
        assert_eq!(
            plan.base_options.lang,
            crate::api::LanguageSpec::Resolved(LanguageCode3::eng())
        );
        assert_eq!(plan.base_options.num_speakers, 3);
        assert!(!plan.base_options.with_utseg);
        assert!(plan.base_options.with_morphosyntax);
        assert_eq!(
            plan.base_options.cache_policies,
            TranscribeCachePolicies::uniform(crate::params::CachePolicy::SkipCache)
        );
        assert!(plan.should_merge_abbrev);
    }

    #[test]
    fn transcribe_s_plan_defaults_to_pyannote_ai_precision_2() {
        let snapshot = make_snapshot(
            ReleasedCommand::TranscribeS,
            CommandOptions::TranscribeS(TranscribeCommand {
                common: CommonOptions::default(),
                asr_engine: AsrEngineName::RevAi,
                diarize: true,
                wor: false.into(),
                merge_abbrev: false.into(),
                batch_size: 8,
                utseg_fallback: false.into(),
            }),
            BTreeMap::new(),
        );

        let plan = TranscribeDispatchPlan::from_job(&snapshot, &ServerConfig::default())
            .expect("transcribe_s plan");

        assert!(matches!(plan.base_options.backend, AsrBackend::RustRevAi));
        assert!(plan.base_options.diarize);
        assert_eq!(
            plan.base_options.speaker_backend,
            Some(SpeakerBackendV2::PyannoteAi)
        );
        assert_eq!(
            plan.base_options.lang,
            crate::api::LanguageSpec::Resolved(LanguageCode3::eng())
        );
        assert_eq!(plan.base_options.num_speakers, 3);
        assert!(plan.base_options.with_utseg);
        assert!(!plan.base_options.with_morphosyntax);
        assert_eq!(
            plan.base_options.cache_policies,
            TranscribeCachePolicies::uniform(crate::params::CachePolicy::UseCache)
        );
        assert!(!plan.should_merge_abbrev);
    }

    #[test]
    fn transcribe_plan_preserves_required_cache_policy() {
        let snapshot = make_snapshot(
            ReleasedCommand::Transcribe,
            CommandOptions::Transcribe(TranscribeCommand {
                common: CommonOptions {
                    require_media_cache: true,
                    ..Default::default()
                },
                asr_engine: AsrEngineName::RevAi,
                diarize: true,
                wor: false.into(),
                merge_abbrev: false.into(),
                batch_size: 8,
                utseg_fallback: false.into(),
            }),
            BTreeMap::new(),
        );

        let plan = TranscribeDispatchPlan::from_job(&snapshot, &ServerConfig::default())
            .expect("transcribe plan");

        assert_eq!(
            plan.base_options.cache_policies,
            TranscribeCachePolicies::uniform(crate::params::CachePolicy::RequireCache)
        );
    }

    #[test]
    fn transcribe_plan_can_refresh_rev_without_refreshing_speaker_evidence() {
        let snapshot = make_snapshot(
            ReleasedCommand::TranscribeS,
            CommandOptions::TranscribeS(TranscribeCommand {
                common: CommonOptions {
                    override_media_cache_tasks: vec!["rev_asr_evidence".to_owned()],
                    ..Default::default()
                },
                asr_engine: AsrEngineName::RevAi,
                diarize: true,
                wor: false.into(),
                merge_abbrev: false.into(),
                batch_size: 8,
                utseg_fallback: false.into(),
            }),
            BTreeMap::new(),
        );

        let plan = TranscribeDispatchPlan::from_job(&snapshot, &ServerConfig::default())
            .expect("transcribe plan");

        assert_eq!(
            plan.base_options.cache_policies.rev_asr,
            crate::params::CachePolicy::SkipCache
        );
        assert_eq!(
            plan.base_options.cache_policies.speaker,
            crate::params::CachePolicy::UseCache
        );
    }

    #[test]
    fn transcribe_plan_can_refresh_speaker_without_refreshing_rev_evidence() {
        let snapshot = make_snapshot(
            ReleasedCommand::TranscribeS,
            CommandOptions::TranscribeS(TranscribeCommand {
                common: CommonOptions {
                    override_media_cache_tasks: vec!["speaker_diarization_raw_evidence".to_owned()],
                    ..Default::default()
                },
                asr_engine: AsrEngineName::RevAi,
                diarize: true,
                wor: false.into(),
                merge_abbrev: false.into(),
                batch_size: 8,
                utseg_fallback: false.into(),
            }),
            BTreeMap::new(),
        );

        let plan = TranscribeDispatchPlan::from_job(&snapshot, &ServerConfig::default())
            .expect("transcribe plan");

        assert_eq!(
            plan.base_options.cache_policies.rev_asr,
            crate::params::CachePolicy::UseCache
        );
        assert_eq!(
            plan.base_options.cache_policies.speaker,
            crate::params::CachePolicy::SkipCache
        );
    }

    #[test]
    fn align_plan_keeps_fa_and_utr_cache_policies_distinct() {
        let snapshot = make_snapshot(
            ReleasedCommand::Align,
            CommandOptions::Align(AlignOptions {
                common: CommonOptions {
                    override_media_cache_tasks: vec!["utr_asr".to_owned()],
                    ..Default::default()
                },
                ..AlignOptions::default()
            }),
            BTreeMap::new(),
        );

        let plan =
            FaDispatchPlan::from_job(&snapshot, &ServerConfig::default()).expect("align plan");

        assert_eq!(
            plan.options.fa_params.cache_policy,
            crate::params::CachePolicy::UseCache
        );
        assert_eq!(plan.utr_cache_policy, crate::params::CachePolicy::SkipCache);
    }

    #[test]
    fn transcribe_s_plan_honors_explicit_local_pyannote_override() {
        let mut common = CommonOptions::default();
        common.engine_overrides.speaker = Some(crate::options::SpeakerEngineName::Pyannote);
        let snapshot = make_snapshot(
            ReleasedCommand::TranscribeS,
            CommandOptions::TranscribeS(TranscribeCommand {
                common,
                asr_engine: AsrEngineName::RevAi,
                diarize: true,
                wor: false.into(),
                merge_abbrev: false.into(),
                batch_size: 8,
                utseg_fallback: false.into(),
            }),
            BTreeMap::new(),
        );

        let plan = TranscribeDispatchPlan::from_job(&snapshot, &ServerConfig::default())
            .expect("transcribe_s plan");

        assert_eq!(
            plan.base_options.speaker_backend,
            Some(SpeakerBackendV2::Pyannote)
        );
    }

    #[test]
    fn benchmark_plan_builds_rust_owned_transcribe_options() {
        let snapshot = make_snapshot(
            ReleasedCommand::Benchmark,
            CommandOptions::Benchmark(BenchmarkOptions {
                common: CommonOptions {
                    override_media_cache: true,
                    ..Default::default()
                },
                asr_engine: AsrEngineName::RevAi,
                wor: true.into(),
                merge_abbrev: true.into(),
            }),
            BTreeMap::new(),
        );

        let plan = BenchmarkDispatchPlan::from_job(&snapshot, &ServerConfig::default())
            .expect("benchmark plan");

        assert!(matches!(plan.base_options.backend, AsrBackend::RustRevAi));
        assert_eq!(plan.base_options.num_speakers, 3);
        assert!(!plan.base_options.with_utseg);
        assert!(!plan.base_options.with_morphosyntax);
        assert!(plan.base_options.write_wor);
        assert!(plan.should_merge_abbrev);
        assert!(plan.mwt.is_empty());
    }

    #[test]
    fn media_analysis_plan_reads_opensmile_feature_set() {
        let snapshot = make_snapshot(
            ReleasedCommand::Opensmile,
            CommandOptions::Opensmile(OpensmileOptions {
                common: CommonOptions::default(),
                feature_set: "ComParE_2016".into(),
            }),
            BTreeMap::new(),
        );

        // Pin memory_tier so resolved_memory_tier() does not call
        // MemoryTier::detect() (which reads live host RAM and is not
        // mockable). The plan's `worker_bootstrap` is derived from
        // the resolved tier; without pinning, this assertion shifts
        // between dev machines (Large/Fleet → Profile) and small CI
        // runners (Small → Task). The expected kernel plan derives from this
        // same explicit configuration.
        let cfg = ServerConfig {
            memory_tier: Some(crate::types::runtime::MemoryTierKind::Large),
            ..Default::default()
        };
        let plan =
            MediaAnalysisDispatchPlan::from_job(&snapshot, &cfg).expect("media analysis plan");

        assert_eq!(
            plan,
            MediaAnalysisDispatchPlan::Opensmile {
                kernel_plan: CommandKernelPlan::for_command_with_policy(
                    ReleasedCommand::Opensmile,
                    1,
                    &crate::host_policy::HostExecutionPolicy::from_server_config(&cfg),
                ),
                feature_set: "ComParE_2016".into(),
            }
        );
    }
}
