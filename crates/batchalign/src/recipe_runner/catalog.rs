//! Static catalog of recipe-runner command metadata.
//!
//! This file holds the `CatalogEntry` table: the single declaration of what
//! every released command is. The stage recipes it points at live in
//! `recipes.rs`.

use crate::api::{ContentType, ReleasedCommand};
use crate::worker::InferTask;

use super::command_spec::{
    CapabilityPlan, CapabilitySurface, CatalogEntry, CommandFamily, PlannerKind,
};
use super::materialize::{FileNamingPolicy, OutputPolicy, SidecarPolicy, StemRewrite};
use super::recipe::ExecutionMode;
use super::recipes::{
    ALIGN_RECIPE, AVQI_RECIPE, BENCHMARK_RECIPE, COMPARE_RECIPE, COREF_RECIPE, DIARIZE_RECIPE,
    MORPHOTAG_RECIPE, OPENSMILE_RECIPE, TRANSCRIBE_RECIPE, TRANSCRIBE_S_RECIPE, TRANSLATE_RECIPE,
    UTSEG_RECIPE,
};

const ASR_TASKS: &[InferTask] = &[InferTask::Asr];
const ASR_AND_SPEAKER_TASKS: &[InferTask] = &[InferTask::Asr, InferTask::Speaker];
const MORPHOSYNTAX_TASKS: &[InferTask] = &[InferTask::Morphosyntax];
const UTSEG_TASKS: &[InferTask] = &[InferTask::Utseg];
const TRANSLATE_TASKS: &[InferTask] = &[InferTask::Translate];
const COREF_TASKS: &[InferTask] = &[InferTask::Coref];
const FA_TASKS: &[InferTask] = &[InferTask::Fa];
const OPENSMILE_TASKS: &[InferTask] = &[InferTask::Opensmile];
const AVQI_TASKS: &[InferTask] = &[InferTask::Avqi];
const DIARIZE_TASKS: &[InferTask] = &[InferTask::Speaker];
const BENCHMARK_TASKS: &[InferTask] = &[InferTask::Asr, InferTask::Morphosyntax];

const NO_SIDECARS: &[SidecarPolicy] = &[];
const COMPARE_SIDECARS: &[SidecarPolicy] = &[SidecarPolicy {
    naming: FileNamingPolicy::ReplaceExtension("compare.csv"),
    content_type: ContentType::Csv,
}];

const COMMAND_SPECS: &[CatalogEntry] = &[
    CatalogEntry {
        command: ReleasedCommand::Compare,
        family: CommandFamily::ReferenceProjection,
        planner: PlannerKind::ComparePairs,
        execution_mode: ExecutionMode::ReferenceProjection,
        capabilities: CapabilityPlan {
            infer_tasks: MORPHOSYNTAX_TASKS,
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
        capabilities: CapabilityPlan {
            infer_tasks: BENCHMARK_TASKS,
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
        command: ReleasedCommand::Transcribe,
        family: CommandFamily::AudioSequential,
        planner: PlannerKind::AudioInputs,
        execution_mode: ExecutionMode::SequentialPerUnit,
        capabilities: CapabilityPlan {
            infer_tasks: ASR_TASKS,
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
        capabilities: CapabilityPlan {
            infer_tasks: ASR_AND_SPEAKER_TASKS,
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
        command: ReleasedCommand::Align,
        family: CommandFamily::AudioSequential,
        planner: PlannerKind::AudioInputs,
        execution_mode: ExecutionMode::SequentialPerUnit,
        capabilities: CapabilityPlan {
            infer_tasks: FA_TASKS,
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
        command: ReleasedCommand::Morphotag,
        family: CommandFamily::BatchedText,
        planner: PlannerKind::TextInputs,
        execution_mode: ExecutionMode::BatchedStage,
        capabilities: CapabilityPlan {
            infer_tasks: MORPHOSYNTAX_TASKS,
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
        capabilities: CapabilityPlan {
            infer_tasks: UTSEG_TASKS,
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
        capabilities: CapabilityPlan {
            infer_tasks: TRANSLATE_TASKS,
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
        capabilities: CapabilityPlan {
            infer_tasks: COREF_TASKS,
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
        command: ReleasedCommand::Opensmile,
        family: CommandFamily::MediaAnalysis,
        planner: PlannerKind::MediaAnalysisInputs,
        execution_mode: ExecutionMode::SequentialPerUnit,
        capabilities: CapabilityPlan {
            infer_tasks: OPENSMILE_TASKS,
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
        capabilities: CapabilityPlan {
            infer_tasks: AVQI_TASKS,
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
        capabilities: CapabilityPlan {
            infer_tasks: DIARIZE_TASKS,
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
