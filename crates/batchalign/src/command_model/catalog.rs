use crate::ReleasedCommand;
use crate::commands::spec::{
    CommandCapabilityKind, CommandDefinition, CommandIoProfile, CommandWorkflowDescriptor,
    RunnerDispatchKind,
};
use crate::recipe_runner::catalog::recipe_command_catalog;
use crate::worker::InferTask;

use super::{CatalogEntry, CommandFamily};

/// Return the authoritative command spec for one released command.
pub(crate) fn command_spec(command: ReleasedCommand) -> &'static CatalogEntry {
    // Catalog invariant: every `ReleasedCommand` variant has a
    // matching `CommandSpec` in `recipe_command_catalog()`. Adding a
    // new released command without its spec is a compile-time-visible
    // omission caught by the catalog test in
    // `crates/batchalign/src/recipe_runner/catalog.rs`.
    #[allow(clippy::expect_used)]
    command_specs()
        .iter()
        .find(|spec| spec.command == command)
        .expect("released command missing authoritative command spec")
}

/// Return the authoritative command specs for all released commands.
pub(crate) fn command_specs() -> &'static [CatalogEntry] {
    recipe_command_catalog()
}

/// Return the legacy command definition derived from the authoritative command
/// spec for one released command.
pub(crate) fn legacy_command_definition(command: ReleasedCommand) -> CommandDefinition {
    let spec = command_spec(command);
    let descriptor = legacy_command_descriptor(command);
    CommandDefinition {
        descriptor,
        execution_shape: execution_shape_for(spec.family),
    }
}

/// Return the legacy workflow descriptor derived from the authoritative command
/// spec for one released command.
pub(crate) fn legacy_command_descriptor(command: ReleasedCommand) -> CommandWorkflowDescriptor {
    let spec = command_spec(command);
    CommandWorkflowDescriptor {
        command: spec.command,
        family: execution_shape_for(spec.family).workflow_family(),
        infer_task: primary_infer_task(spec),
        capability_kind: capability_kind_for(spec.command),
        io_profile: io_profile_for(spec.command),
        runner_dispatch_kind: runner_dispatch_kind_for(spec.command),
    }
}

fn primary_infer_task(spec: &CatalogEntry) -> InferTask {
    // Catalog invariant: every spec carries a non-empty
    // `infer_tasks` list. Empty `infer_tasks` would mean the command
    // doesn't dispatch to any inference task, which is invalid by
    // construction, and the catalog test rejects it.
    #[allow(clippy::expect_used)]
    spec.capabilities
        .infer_tasks
        .first()
        .copied()
        .expect("released command must advertise at least one infer task")
}

fn capability_kind_for(command: ReleasedCommand) -> CommandCapabilityKind {
    match command {
        ReleasedCommand::Transcribe | ReleasedCommand::TranscribeS | ReleasedCommand::Benchmark => {
            CommandCapabilityKind::ServerComposed
        }
        _ => CommandCapabilityKind::DirectInfer,
    }
}

fn io_profile_for(command: ReleasedCommand) -> CommandIoProfile {
    match command {
        ReleasedCommand::Align
        | ReleasedCommand::Transcribe
        | ReleasedCommand::TranscribeS
        | ReleasedCommand::Benchmark
        | ReleasedCommand::Opensmile
        | ReleasedCommand::Avqi
        | ReleasedCommand::Diarize => CommandIoProfile::PathsModeAudio,
        _ => CommandIoProfile::PathsModeText,
    }
}

fn runner_dispatch_kind_for(command: ReleasedCommand) -> RunnerDispatchKind {
    match command {
        ReleasedCommand::Align => RunnerDispatchKind::ForcedAlignment,
        ReleasedCommand::Transcribe | ReleasedCommand::TranscribeS => {
            RunnerDispatchKind::TranscribeAudioInfer
        }
        ReleasedCommand::Benchmark => RunnerDispatchKind::BenchmarkAudioInfer,
        ReleasedCommand::Opensmile | ReleasedCommand::Avqi | ReleasedCommand::Diarize => {
            RunnerDispatchKind::MediaAnalysisV2
        }
        _ => RunnerDispatchKind::BatchedTextInfer,
    }
}

fn execution_shape_for(family: CommandFamily) -> crate::commands::spec::CommandExecutionShape {
    match family {
        CommandFamily::ReferenceProjection => {
            crate::commands::spec::CommandExecutionShape::ReferenceProjection
        }
        CommandFamily::AudioSequential => {
            crate::commands::spec::CommandExecutionShape::AudioSequential
        }
        CommandFamily::BatchedText => crate::commands::spec::CommandExecutionShape::BatchedText,
        CommandFamily::Composite => crate::commands::spec::CommandExecutionShape::Composite,
        CommandFamily::MediaAnalysis => crate::commands::spec::CommandExecutionShape::MediaAnalysis,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_family::WorkflowFamily;
    use crate::commands::spec::{
        BatchingPolicy, ConstrainedHostPolicy, ModelSharingPolicy, ParallelismPolicy, ResourceLane,
        SchedulingPolicy, WarmupPolicy,
    };
    use crate::recipe_runner::materialize::{FileNamingPolicy, StemRewrite};

    /// Output filenames are a user-visible contract, and after the 2026-07-28
    /// cleanup there is exactly ONE representation of them: the `output_policy`
    /// declared per `CatalogEntry`.
    ///
    /// There used to be a second, `CommandOutputPathKind`, derived by matching
    /// on the command name with a catch-all `_ => PreserveInputName`. It was
    /// read by nothing in production and it contradicted the declared policy for
    /// four commands (benchmark, opensmile, avqi, diarize). Worse, it could not
    /// express `RewriteStem` at all, so for diarize (`X.turns.json`) it was
    /// structurally incapable of stating the truth. It was deleted rather than
    /// synchronised. This test pins the survivor so the contract cannot drift
    /// silently.
    #[test]
    fn declared_output_naming_is_stable() {
        // Matched on the ENUM with no catch-all arm: adding a `ReleasedCommand`
        // fails to compile here until its output naming is stated. A `_ =>`
        // default would reintroduce, in miniature, the silent-default hazard
        // the deleted `output_path_kind_for` embodied.
        for command in ReleasedCommand::ALL {
            let expected = match command {
                ReleasedCommand::Transcribe
                | ReleasedCommand::TranscribeS
                | ReleasedCommand::Benchmark => FileNamingPolicy::ReplaceExtension("cha"),
                ReleasedCommand::Opensmile => FileNamingPolicy::RewriteStem(StemRewrite {
                    strip_suffix: None,
                    append_suffix: ".opensmile",
                    extension: "csv",
                }),
                ReleasedCommand::Avqi => FileNamingPolicy::RewriteStem(StemRewrite {
                    strip_suffix: Some(".cs"),
                    append_suffix: ".avqi",
                    extension: "txt",
                }),
                ReleasedCommand::Diarize => FileNamingPolicy::RewriteStem(StemRewrite {
                    strip_suffix: None,
                    append_suffix: ".turns",
                    extension: "json",
                }),
                // The CHAT transforms all rewrite their input in place.
                ReleasedCommand::Morphotag
                | ReleasedCommand::Utseg
                | ReleasedCommand::Translate
                | ReleasedCommand::Coref
                | ReleasedCommand::Align
                | ReleasedCommand::Compare => FileNamingPolicy::PreserveInput,
            };
            assert_eq!(
                command_spec(command).output_policy.primary,
                expected,
                "output naming changed for {command}"
            );
        }
    }

    /// Everything the rest of the app reads about ONE released command, in one
    /// named record.
    ///
    /// Written as a struct rather than a tuple deliberately: a tuple of five
    /// same-shaped enums is exactly the kind of positional seam that lets a
    /// transcription error pass review (charter rule: no tuple-packed domain
    /// seams).
    struct CommandMetadataPin {
        command: ReleasedCommand,
        family: CommandFamily,
        capability_kind: CommandCapabilityKind,
        io_profile: CommandIoProfile,
        runner_dispatch_kind: RunnerDispatchKind,
        primary_infer_task: InferTask,
    }

    /// The pinned metadata for every released command.
    ///
    /// This is a CHARACTERIZATION table: it states what the catalog resolves to
    /// TODAY, so that a restructuring of where those values are stated cannot
    /// change what they are. Three of these five fields are currently produced
    /// by `match` arms with a catch-all `_ =>` default, which is precisely why
    /// they need pinning: a catch-all silently absorbs a command that should
    /// have been given an explicit answer (the failure mode that produced the
    /// deleted `output_path_kind_for`, documented above).
    fn command_metadata_pins() -> Vec<CommandMetadataPin> {
        // Matched on the enum with NO catch-all arm, so adding a released
        // command fails to compile here until its metadata is stated.
        ReleasedCommand::ALL
            .into_iter()
            .map(|command| match command {
                ReleasedCommand::Compare => CommandMetadataPin {
                    command,
                    family: CommandFamily::ReferenceProjection,
                    capability_kind: CommandCapabilityKind::DirectInfer,
                    io_profile: CommandIoProfile::PathsModeText,
                    runner_dispatch_kind: RunnerDispatchKind::BatchedTextInfer,
                    primary_infer_task: InferTask::Morphosyntax,
                },
                ReleasedCommand::Benchmark => CommandMetadataPin {
                    command,
                    family: CommandFamily::Composite,
                    capability_kind: CommandCapabilityKind::ServerComposed,
                    io_profile: CommandIoProfile::PathsModeAudio,
                    runner_dispatch_kind: RunnerDispatchKind::BenchmarkAudioInfer,
                    primary_infer_task: InferTask::Asr,
                },
                ReleasedCommand::Transcribe => CommandMetadataPin {
                    command,
                    family: CommandFamily::AudioSequential,
                    capability_kind: CommandCapabilityKind::ServerComposed,
                    io_profile: CommandIoProfile::PathsModeAudio,
                    runner_dispatch_kind: RunnerDispatchKind::TranscribeAudioInfer,
                    primary_infer_task: InferTask::Asr,
                },
                ReleasedCommand::TranscribeS => CommandMetadataPin {
                    command,
                    family: CommandFamily::AudioSequential,
                    capability_kind: CommandCapabilityKind::ServerComposed,
                    io_profile: CommandIoProfile::PathsModeAudio,
                    runner_dispatch_kind: RunnerDispatchKind::TranscribeAudioInfer,
                    primary_infer_task: InferTask::Asr,
                },
                ReleasedCommand::Align => CommandMetadataPin {
                    command,
                    family: CommandFamily::AudioSequential,
                    capability_kind: CommandCapabilityKind::DirectInfer,
                    io_profile: CommandIoProfile::PathsModeAudio,
                    runner_dispatch_kind: RunnerDispatchKind::ForcedAlignment,
                    primary_infer_task: InferTask::Fa,
                },
                ReleasedCommand::Morphotag => CommandMetadataPin {
                    command,
                    family: CommandFamily::BatchedText,
                    capability_kind: CommandCapabilityKind::DirectInfer,
                    io_profile: CommandIoProfile::PathsModeText,
                    runner_dispatch_kind: RunnerDispatchKind::BatchedTextInfer,
                    primary_infer_task: InferTask::Morphosyntax,
                },
                ReleasedCommand::Utseg => CommandMetadataPin {
                    command,
                    family: CommandFamily::BatchedText,
                    capability_kind: CommandCapabilityKind::DirectInfer,
                    io_profile: CommandIoProfile::PathsModeText,
                    runner_dispatch_kind: RunnerDispatchKind::BatchedTextInfer,
                    primary_infer_task: InferTask::Utseg,
                },
                ReleasedCommand::Translate => CommandMetadataPin {
                    command,
                    family: CommandFamily::BatchedText,
                    capability_kind: CommandCapabilityKind::DirectInfer,
                    io_profile: CommandIoProfile::PathsModeText,
                    runner_dispatch_kind: RunnerDispatchKind::BatchedTextInfer,
                    primary_infer_task: InferTask::Translate,
                },
                ReleasedCommand::Coref => CommandMetadataPin {
                    command,
                    family: CommandFamily::BatchedText,
                    capability_kind: CommandCapabilityKind::DirectInfer,
                    io_profile: CommandIoProfile::PathsModeText,
                    runner_dispatch_kind: RunnerDispatchKind::BatchedTextInfer,
                    primary_infer_task: InferTask::Coref,
                },
                ReleasedCommand::Opensmile => CommandMetadataPin {
                    command,
                    family: CommandFamily::MediaAnalysis,
                    capability_kind: CommandCapabilityKind::DirectInfer,
                    io_profile: CommandIoProfile::PathsModeAudio,
                    runner_dispatch_kind: RunnerDispatchKind::MediaAnalysisV2,
                    primary_infer_task: InferTask::Opensmile,
                },
                ReleasedCommand::Avqi => CommandMetadataPin {
                    command,
                    family: CommandFamily::MediaAnalysis,
                    capability_kind: CommandCapabilityKind::DirectInfer,
                    io_profile: CommandIoProfile::PathsModeAudio,
                    runner_dispatch_kind: RunnerDispatchKind::MediaAnalysisV2,
                    primary_infer_task: InferTask::Avqi,
                },
                ReleasedCommand::Diarize => CommandMetadataPin {
                    command,
                    family: CommandFamily::MediaAnalysis,
                    capability_kind: CommandCapabilityKind::DirectInfer,
                    io_profile: CommandIoProfile::PathsModeAudio,
                    runner_dispatch_kind: RunnerDispatchKind::MediaAnalysisV2,
                    primary_infer_task: InferTask::Speaker,
                },
            })
            .collect()
    }

    /// The whole per-command metadata surface, pinned command by command.
    ///
    /// Every consumer in the crate (capability advertisement, worker targeting,
    /// runner routing, the CLI's paths-mode decision, kernel planning) reads one
    /// of these five values, so holding all five constant is what makes a
    /// restructuring of the catalog provably behaviour-preserving.
    #[test]
    fn per_command_metadata_is_stable() {
        for pin in command_metadata_pins() {
            let spec = command_spec(pin.command);
            let descriptor = legacy_command_descriptor(pin.command);
            assert_eq!(spec.family, pin.family, "family for {}", pin.command);
            assert_eq!(
                descriptor.capability_kind, pin.capability_kind,
                "capability kind for {}",
                pin.command
            );
            assert_eq!(
                descriptor.io_profile, pin.io_profile,
                "io profile for {}",
                pin.command
            );
            assert_eq!(
                descriptor.runner_dispatch_kind, pin.runner_dispatch_kind,
                "runner dispatch kind for {}",
                pin.command
            );
            assert_eq!(
                descriptor.infer_task, pin.primary_infer_task,
                "primary infer task for {}",
                pin.command
            );
        }
    }

    /// Everything a command FAMILY implies about runtime policy, in one record.
    #[derive(Debug, PartialEq, Eq)]
    struct FamilyPolicyPin {
        workflow: WorkflowFamily,
        scheduling: SchedulingPolicy,
        model_sharing: ModelSharingPolicy,
        batching: BatchingPolicy,
        parallelism: ParallelismPolicy,
        resource_lane: ResourceLane,
        constrained_host: ConstrainedHostPolicy,
        warmup: WarmupPolicy,
    }

    /// The policy vocabulary a command family implies, pinned family by family.
    ///
    /// The per-command table above pins WHICH family each command is in; this
    /// pins what being in that family MEANS. Together they cover the whole
    /// derivation, which lets the two identical five-variant enums
    /// (`CommandFamily` and `CommandExecutionShape`) be collapsed into one
    /// without silently re-mapping a family onto different runtime policy.
    #[test]
    fn family_policy_derivation_is_stable() {
        // No catch-all: a new family must state its policy here.
        for family in [
            CommandFamily::BatchedText,
            CommandFamily::ReferenceProjection,
            CommandFamily::AudioSequential,
            CommandFamily::MediaAnalysis,
            CommandFamily::Composite,
        ] {
            let shape = execution_shape_for(family);
            let expected = match family {
                CommandFamily::BatchedText => FamilyPolicyPin {
                    workflow: WorkflowFamily::CrossFileBatchTransform,
                    scheduling: SchedulingPolicy::CrossFileBatch,
                    model_sharing: ModelSharingPolicy::SharedWarmWorkers,
                    batching: BatchingPolicy::CrossFileBatch,
                    parallelism: ParallelismPolicy::SingleDispatchPerJob,
                    resource_lane: ResourceLane::CpuBound,
                    constrained_host: ConstrainedHostPolicy::SequentialFallback,
                    warmup: WarmupPolicy::BackgroundEligible,
                },
                CommandFamily::ReferenceProjection => FamilyPolicyPin {
                    workflow: WorkflowFamily::ReferenceProjection,
                    scheduling: SchedulingPolicy::ReferenceProjection,
                    model_sharing: ModelSharingPolicy::SharedWarmWorkers,
                    batching: BatchingPolicy::PairedInputs,
                    parallelism: ParallelismPolicy::SingleDispatchPerJob,
                    resource_lane: ResourceLane::Mixed,
                    constrained_host: ConstrainedHostPolicy::SequentialFallback,
                    warmup: WarmupPolicy::BackgroundEligible,
                },
                CommandFamily::AudioSequential => FamilyPolicyPin {
                    workflow: WorkflowFamily::PerFileTransform,
                    scheduling: SchedulingPolicy::PerFileAudio,
                    model_sharing: ModelSharingPolicy::SharedWarmWorkers,
                    batching: BatchingPolicy::InternalStageBatching,
                    parallelism: ParallelismPolicy::BoundedFileWorkers,
                    resource_lane: ResourceLane::GpuHeavy,
                    constrained_host: ConstrainedHostPolicy::SequentialFallback,
                    warmup: WarmupPolicy::BackgroundEligible,
                },
                CommandFamily::MediaAnalysis => FamilyPolicyPin {
                    workflow: WorkflowFamily::PerFileTransform,
                    scheduling: SchedulingPolicy::PerFileMediaAnalysis,
                    model_sharing: ModelSharingPolicy::SharedWarmWorkers,
                    batching: BatchingPolicy::None,
                    parallelism: ParallelismPolicy::BoundedFileWorkers,
                    resource_lane: ResourceLane::IoBound,
                    constrained_host: ConstrainedHostPolicy::SequentialFallback,
                    warmup: WarmupPolicy::LazyOnDemand,
                },
                CommandFamily::Composite => FamilyPolicyPin {
                    workflow: WorkflowFamily::Composite,
                    scheduling: SchedulingPolicy::Composite,
                    model_sharing: ModelSharingPolicy::DelegatedToSubcommands,
                    batching: BatchingPolicy::None,
                    parallelism: ParallelismPolicy::DelegatedToSubcommands,
                    resource_lane: ResourceLane::Mixed,
                    constrained_host: ConstrainedHostPolicy::DelegatedToSubcommands,
                    warmup: WarmupPolicy::DelegatedToSubcommands,
                },
            };
            let actual = FamilyPolicyPin {
                workflow: shape.workflow_family(),
                scheduling: shape.scheduling_policy(),
                model_sharing: shape.model_sharing_policy(),
                batching: shape.batching_policy(),
                parallelism: shape.parallelism_policy(),
                resource_lane: shape.resource_lane(),
                constrained_host: shape.constrained_host_policy(),
                warmup: shape.warmup_policy(),
            };
            assert_eq!(actual, expected, "policy derivation for {family:?}");
            assert!(
                shape.uses_host_memory_gate(),
                "host memory gate must stay on for {family:?}"
            );
        }
    }

    #[test]
    fn command_specs_are_unique() {
        let mut names: Vec<_> = command_specs().iter().map(|spec| spec.command).collect();
        let original_len = names.len();
        names.sort_unstable_by_key(|command| command.as_ref().to_owned());
        names.dedup();
        assert_eq!(names.len(), original_len);
    }

    #[test]
    fn compare_legacy_descriptor_matches_reference_projection_shape() {
        let definition = legacy_command_definition(ReleasedCommand::Compare);
        assert_eq!(
            definition.scheduling_policy(),
            SchedulingPolicy::ReferenceProjection
        );
        assert_eq!(definition.batching_policy(), BatchingPolicy::PairedInputs);
    }
}
