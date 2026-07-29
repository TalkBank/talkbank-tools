//! Static catalog of recipe-runner command metadata.
//!
//! This file holds the `CatalogEntry` table: the single declaration of what
//! every released command is. The stage recipes it points at live in
//! `recipes.rs`.

use crate::api::{ContentType, ReleasedCommand};
use crate::worker::InferTask;

use super::command_spec::{
    CapabilityPlan, CapabilitySurface, CatalogEntry, CommandCapabilityKind, CommandFamily,
    CommandIoProfile, PlannerKind, RunnerDispatchKind,
};
use super::materialize::{FileNamingPolicy, OutputPolicy, SidecarPolicy, StemRewrite};
use super::recipe::ExecutionMode;
use super::recipes::{
    ALIGN_RECIPE, AVQI_RECIPE, BENCHMARK_RECIPE, COMPARE_RECIPE, COREF_RECIPE, DIARIZE_RECIPE,
    MORPHOTAG_RECIPE, OPENSMILE_RECIPE, TRANSCRIBE_RECIPE, TRANSCRIBE_S_RECIPE, TRANSLATE_RECIPE,
    UTSEG_RECIPE,
};


const NO_SIDECARS: &[SidecarPolicy] = &[];
const COMPARE_SIDECARS: &[SidecarPolicy] = &[SidecarPolicy {
    naming: FileNamingPolicy::ReplaceExtension("compare.csv"),
    content_type: ContentType::Csv,
}];

/// Every released command, declared once.
///
/// ORDER IS SIGNIFICANT: `capability::derive_command_capabilities` walks this
/// table to build the capability list served by `/health` and rendered by the
/// dashboard, so this sequence is a user-visible contract. It was previously
/// held in a second list (`commands/catalog.rs::RELEASED_COMMAND_ORDER`) whose
/// order DIFFERED from this table's; the two were reconciled onto this one on
/// 2026-07-29 by adopting the advertised order here.
const COMMAND_SPECS: &[CatalogEntry] = &[
    CatalogEntry {
        command: ReleasedCommand::Morphotag,
        family: CommandFamily::BatchedText,
        planner: PlannerKind::TextInputs,
        execution_mode: ExecutionMode::BatchedStage,
        capability_kind: CommandCapabilityKind::DirectInfer,
        io_profile: CommandIoProfile::PathsModeText,
        runner_dispatch_kind: RunnerDispatchKind::BatchedTextInfer,
        capabilities: CapabilityPlan {
            primary_infer_task: InferTask::Morphosyntax,
            additional_infer_tasks: &[],
            surface: CapabilitySurface::RecipeOwned,
        },
        output_policy: OutputPolicy {
            primary: FileNamingPolicy::PreserveInput,
            primary_content_type: ContentType::Chat,
            sidecars: NO_SIDECARS,
        },
        recipe: &MORPHOTAG_RECIPE,
    },
    CatalogEntry {
        command: ReleasedCommand::Utseg,
        family: CommandFamily::BatchedText,
        planner: PlannerKind::TextInputs,
        execution_mode: ExecutionMode::BatchedStage,
        capability_kind: CommandCapabilityKind::DirectInfer,
        io_profile: CommandIoProfile::PathsModeText,
        runner_dispatch_kind: RunnerDispatchKind::BatchedTextInfer,
        capabilities: CapabilityPlan {
            primary_infer_task: InferTask::Utseg,
            additional_infer_tasks: &[],
            surface: CapabilitySurface::RecipeOwned,
        },
        output_policy: OutputPolicy {
            primary: FileNamingPolicy::PreserveInput,
            primary_content_type: ContentType::Chat,
            sidecars: NO_SIDECARS,
        },
        recipe: &UTSEG_RECIPE,
    },
    CatalogEntry {
        command: ReleasedCommand::Translate,
        family: CommandFamily::BatchedText,
        planner: PlannerKind::TextInputs,
        execution_mode: ExecutionMode::BatchedStage,
        capability_kind: CommandCapabilityKind::DirectInfer,
        io_profile: CommandIoProfile::PathsModeText,
        runner_dispatch_kind: RunnerDispatchKind::BatchedTextInfer,
        capabilities: CapabilityPlan {
            primary_infer_task: InferTask::Translate,
            additional_infer_tasks: &[],
            surface: CapabilitySurface::RecipeOwned,
        },
        output_policy: OutputPolicy {
            primary: FileNamingPolicy::PreserveInput,
            primary_content_type: ContentType::Chat,
            sidecars: NO_SIDECARS,
        },
        recipe: &TRANSLATE_RECIPE,
    },
    CatalogEntry {
        command: ReleasedCommand::Coref,
        family: CommandFamily::BatchedText,
        planner: PlannerKind::TextInputs,
        execution_mode: ExecutionMode::BatchedStage,
        capability_kind: CommandCapabilityKind::DirectInfer,
        io_profile: CommandIoProfile::PathsModeText,
        runner_dispatch_kind: RunnerDispatchKind::BatchedTextInfer,
        capabilities: CapabilityPlan {
            primary_infer_task: InferTask::Coref,
            additional_infer_tasks: &[],
            surface: CapabilitySurface::RecipeOwned,
        },
        output_policy: OutputPolicy {
            primary: FileNamingPolicy::PreserveInput,
            primary_content_type: ContentType::Chat,
            sidecars: NO_SIDECARS,
        },
        recipe: &COREF_RECIPE,
    },
    CatalogEntry {
        command: ReleasedCommand::Align,
        family: CommandFamily::AudioSequential,
        planner: PlannerKind::AudioInputs,
        execution_mode: ExecutionMode::SequentialPerUnit,
        capability_kind: CommandCapabilityKind::DirectInfer,
        io_profile: CommandIoProfile::PathsModeAudio,
        runner_dispatch_kind: RunnerDispatchKind::ForcedAlignment,
        capabilities: CapabilityPlan {
            primary_infer_task: InferTask::Fa,
            additional_infer_tasks: &[],
            surface: CapabilitySurface::RecipeOwned,
        },
        output_policy: OutputPolicy {
            primary: FileNamingPolicy::PreserveInput,
            primary_content_type: ContentType::Chat,
            sidecars: NO_SIDECARS,
        },
        recipe: &ALIGN_RECIPE,
    },
    CatalogEntry {
        command: ReleasedCommand::Transcribe,
        family: CommandFamily::AudioSequential,
        planner: PlannerKind::AudioInputs,
        execution_mode: ExecutionMode::SequentialPerUnit,
        capability_kind: CommandCapabilityKind::ServerComposed,
        io_profile: CommandIoProfile::PathsModeAudio,
        runner_dispatch_kind: RunnerDispatchKind::TranscribeAudioInfer,
        capabilities: CapabilityPlan {
            primary_infer_task: InferTask::Asr,
            additional_infer_tasks: &[],
            surface: CapabilitySurface::RecipeOwned,
        },
        output_policy: OutputPolicy {
            primary: FileNamingPolicy::ReplaceExtension("cha"),
            primary_content_type: ContentType::Chat,
            sidecars: NO_SIDECARS,
        },
        recipe: &TRANSCRIBE_RECIPE,
    },
    CatalogEntry {
        command: ReleasedCommand::TranscribeS,
        family: CommandFamily::AudioSequential,
        planner: PlannerKind::AudioInputs,
        execution_mode: ExecutionMode::SequentialPerUnit,
        capability_kind: CommandCapabilityKind::ServerComposed,
        io_profile: CommandIoProfile::PathsModeAudio,
        runner_dispatch_kind: RunnerDispatchKind::TranscribeAudioInfer,
        capabilities: CapabilityPlan {
            primary_infer_task: InferTask::Asr,
            additional_infer_tasks: &[InferTask::Speaker],
            surface: CapabilitySurface::RecipeOwned,
        },
        output_policy: OutputPolicy {
            primary: FileNamingPolicy::ReplaceExtension("cha"),
            primary_content_type: ContentType::Chat,
            sidecars: NO_SIDECARS,
        },
        recipe: &TRANSCRIBE_S_RECIPE,
    },
    CatalogEntry {
        command: ReleasedCommand::Compare,
        family: CommandFamily::ReferenceProjection,
        planner: PlannerKind::ComparePairs,
        execution_mode: ExecutionMode::ReferenceProjection,
        capability_kind: CommandCapabilityKind::DirectInfer,
        io_profile: CommandIoProfile::PathsModeText,
        runner_dispatch_kind: RunnerDispatchKind::BatchedTextInfer,
        capabilities: CapabilityPlan {
            primary_infer_task: InferTask::Morphosyntax,
            additional_infer_tasks: &[],
            surface: CapabilitySurface::RecipeOwned,
        },
        output_policy: OutputPolicy {
            primary: FileNamingPolicy::PreserveInput,
            primary_content_type: ContentType::Chat,
            sidecars: COMPARE_SIDECARS,
        },
        recipe: &COMPARE_RECIPE,
    },
    CatalogEntry {
        command: ReleasedCommand::Benchmark,
        family: CommandFamily::Composite,
        planner: PlannerKind::BenchmarkPairs,
        execution_mode: ExecutionMode::Composite,
        capability_kind: CommandCapabilityKind::ServerComposed,
        io_profile: CommandIoProfile::PathsModeAudio,
        runner_dispatch_kind: RunnerDispatchKind::BenchmarkAudioInfer,
        capabilities: CapabilityPlan {
            primary_infer_task: InferTask::Asr,
            additional_infer_tasks: &[InferTask::Morphosyntax],
            surface: CapabilitySurface::Composite,
        },
        output_policy: OutputPolicy {
            primary: FileNamingPolicy::ReplaceExtension("cha"),
            primary_content_type: ContentType::Chat,
            sidecars: COMPARE_SIDECARS,
        },
        recipe: &BENCHMARK_RECIPE,
    },
    CatalogEntry {
        command: ReleasedCommand::Opensmile,
        family: CommandFamily::MediaAnalysis,
        planner: PlannerKind::MediaAnalysisInputs,
        execution_mode: ExecutionMode::SequentialPerUnit,
        capability_kind: CommandCapabilityKind::DirectInfer,
        io_profile: CommandIoProfile::PathsModeAudio,
        runner_dispatch_kind: RunnerDispatchKind::MediaAnalysisV2,
        capabilities: CapabilityPlan {
            primary_infer_task: InferTask::Opensmile,
            additional_infer_tasks: &[],
            surface: CapabilitySurface::RecipeOwned,
        },
        output_policy: OutputPolicy {
            primary: FileNamingPolicy::RewriteStem(StemRewrite {
                strip_suffix: None,
                append_suffix: ".opensmile",
                extension: "csv",
            }),
            primary_content_type: ContentType::Csv,
            sidecars: NO_SIDECARS,
        },
        recipe: &OPENSMILE_RECIPE,
    },
    CatalogEntry {
        command: ReleasedCommand::Avqi,
        family: CommandFamily::MediaAnalysis,
        planner: PlannerKind::MediaAnalysisInputs,
        execution_mode: ExecutionMode::SequentialPerUnit,
        capability_kind: CommandCapabilityKind::DirectInfer,
        io_profile: CommandIoProfile::PathsModeAudio,
        runner_dispatch_kind: RunnerDispatchKind::MediaAnalysisV2,
        capabilities: CapabilityPlan {
            primary_infer_task: InferTask::Avqi,
            additional_infer_tasks: &[],
            surface: CapabilitySurface::RecipeOwned,
        },
        output_policy: OutputPolicy {
            primary: FileNamingPolicy::RewriteStem(StemRewrite {
                strip_suffix: Some(".cs"),
                append_suffix: ".avqi",
                extension: "txt",
            }),
            primary_content_type: ContentType::Text,
            sidecars: NO_SIDECARS,
        },
        recipe: &AVQI_RECIPE,
    },
    CatalogEntry {
        command: ReleasedCommand::Diarize,
        family: CommandFamily::MediaAnalysis,
        planner: PlannerKind::MediaAnalysisInputs,
        execution_mode: ExecutionMode::SequentialPerUnit,
        capability_kind: CommandCapabilityKind::DirectInfer,
        io_profile: CommandIoProfile::PathsModeAudio,
        runner_dispatch_kind: RunnerDispatchKind::MediaAnalysisV2,
        capabilities: CapabilityPlan {
            primary_infer_task: InferTask::Speaker,
            additional_infer_tasks: &[],
            surface: CapabilitySurface::RecipeOwned,
        },
        output_policy: OutputPolicy {
            // `CWS-032-1.mp3` -> `CWS-032-1.turns.json`, the artifact
            // name `chatter rediarize --turns-dir` conventions expect.
            primary: FileNamingPolicy::RewriteStem(StemRewrite {
                strip_suffix: None,
                append_suffix: ".turns",
                extension: "json",
            }),
            primary_content_type: ContentType::Json,
            sidecars: NO_SIDECARS,
        },
        recipe: &DIARIZE_RECIPE,
    },
];

/// Return the static recipe-runner command catalog.
pub(crate) fn recipe_command_catalog() -> &'static [CatalogEntry] {
    COMMAND_SPECS
}

/// Look up one released command in the recipe catalog.
#[allow(dead_code, clippy::expect_used)]
pub(crate) fn recipe_command_spec(command: ReleasedCommand) -> &'static CatalogEntry {
    // Catalog invariant: `COMMAND_SPECS` covers every
    // `ReleasedCommand` variant; the catalog test below
    // (`tests::every_released_command_has_a_spec`) enforces this at
    // build time.
    COMMAND_SPECS
        .iter()
        .find(|spec| spec.command == command)
        .expect("recipe runner command missing catalog entry")
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::api::DisplayPath;
    use crate::recipe_runner::materialize::plan_materialized_files;
    use crate::recipe_runner::recipe::{RecipeStageId, RecipeStagePresence};

    /// Every released command must have a catalog entry.
    ///
    /// `recipe_command_spec` and `command_model::command_spec` both cite this
    /// test by name as the enforcement of that invariant, and until 2026-07-29
    /// it did not exist: the comments claimed a guarantee nothing checked. The
    /// equivalent coverage test did exist, over the second (now deleted)
    /// command-order list in `commands/catalog.rs`, where it recorded why it
    /// mattered: a released command missing from the list silently vanishes from
    /// capability advertisement and dispatch reports "Unknown command" despite a
    /// working CLI subcommand (the 2026-07-10 diarize field failure).
    #[test]
    fn every_released_command_has_a_spec() {
        for command in ReleasedCommand::ALL {
            assert!(
                COMMAND_SPECS.iter().any(|spec| spec.command == command),
                "released command {command} missing from COMMAND_SPECS"
            );
        }
        assert_eq!(
            COMMAND_SPECS.len(),
            ReleasedCommand::ALL.len(),
            "COMMAND_SPECS holds an entry for something that is not a released command"
        );
    }

    #[test]
    fn catalog_entries_are_unique_and_validate() {
        let mut seen = HashSet::new();
        for spec in recipe_command_catalog() {
            assert!(seen.insert(spec.command));
            spec.recipe.validate(spec.command).expect("valid recipe");
            assert_eq!(spec.execution_mode, spec.recipe.mode);
        }
    }

    #[test]
    fn compare_spec_keeps_reference_projection_and_sidecar_output() {
        let spec = recipe_command_spec(ReleasedCommand::Compare);
        assert_eq!(spec.family, CommandFamily::ReferenceProjection);
        assert_eq!(spec.execution_mode, ExecutionMode::ReferenceProjection);
        let outputs = plan_materialized_files(&"sample.cha".into(), spec.output_policy);
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].display_path, DisplayPath::from("sample.cha"));
        assert_eq!(
            outputs[1].display_path,
            DisplayPath::from("sample.compare.csv")
        );
    }

    #[test]
    fn transcribe_recipe_keeps_asr_before_chat_build() {
        let spec = recipe_command_spec(ReleasedCommand::Transcribe);
        assert_eq!(
            spec.recipe.ordered_stage_ids(),
            vec![
                RecipeStageId::PlanWorkUnits,
                RecipeStageId::ResolveAudio,
                RecipeStageId::AsrInfer,
                RecipeStageId::SpeakerDiarization,
                RecipeStageId::AsrPostprocess,
                RecipeStageId::BuildChat,
                RecipeStageId::UtteranceSegmentation,
                RecipeStageId::Morphosyntax,
                RecipeStageId::SerializeChat,
                RecipeStageId::MaterializeOutputs,
            ]
        );
    }

    #[test]
    fn transcribe_s_requires_speaker_stage() {
        let spec = recipe_command_spec(ReleasedCommand::TranscribeS);
        let diarization = spec
            .recipe
            .stages
            .iter()
            .find(|stage| stage.id == RecipeStageId::SpeakerDiarization)
            .expect("speaker stage");
        assert_eq!(diarization.presence, RecipeStagePresence::Required);
    }

    #[test]
    fn media_analysis_specs_match_current_output_filenames() {
        let opensmile = recipe_command_spec(ReleasedCommand::Opensmile);
        let avqi = recipe_command_spec(ReleasedCommand::Avqi);
        let opensmile_outputs =
            plan_materialized_files(&"sample.wav".into(), opensmile.output_policy);
        let avqi_outputs = plan_materialized_files(&"sample.cs.wav".into(), avqi.output_policy);
        assert_eq!(
            opensmile_outputs[0].display_path,
            DisplayPath::from("sample.opensmile.csv")
        );
        assert_eq!(
            avqi_outputs[0].display_path,
            DisplayPath::from("sample.avqi.txt")
        );
    }
}
