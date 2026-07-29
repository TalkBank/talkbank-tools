use crate::ReleasedCommand;
use crate::commands::spec::{
    CommandCapabilityKind, CommandDefinition, CommandIoProfile,
    CommandWorkflowDescriptor, RunnerDispatchKind,
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
    use crate::commands::spec::{BatchingPolicy, SchedulingPolicy};
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
        // Matched on the ENUM, not on the stringified name, and with no
        // catch-all arm: adding a `ReleasedCommand` fails to compile here until
        // its output naming is stated. A `_ =>` default in this very test would
        // reintroduce, in miniature, exactly the silent-default hazard the
        // deleted `output_path_kind_for` embodied. (It did, briefly: the first
        // draft matched strings and let `transcribe_s` fall through.)
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
