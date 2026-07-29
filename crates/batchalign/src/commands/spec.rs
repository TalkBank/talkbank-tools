//! Command-owned catalog types and derived runtime policy.

use crate::ReleasedCommand;
use crate::command_family::WorkflowFamily;
// These three are declared fields of `CatalogEntry` and live with it; the
// compatibility descriptor below only carries them through to consumers that
// have not yet been pointed at the catalog directly.
use crate::recipe_runner::command_spec::{
    CommandCapabilityKind, CommandIoProfile, RunnerDispatchKind,
};
use crate::worker::InferTask;

/// High-level scheduling shape the command expects from the shared kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchedulingPolicy {
    /// One audio/media file at a time, with bounded per-job parallelism.
    PerFileAudio,
    /// Many text files pooled into one or more shared infer batches.
    CrossFileBatch,
    /// One primary file plus one paired reference artifact.
    ReferenceProjection,
    /// The command is built by composing other command-owned flows.
    Composite,
    /// Per-file media analysis over non-CHAT inputs.
    PerFileMediaAnalysis,
}

/// How the command expects model state to be shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModelSharingPolicy {
    /// Reuse warm workers and shared model state whenever possible.
    SharedWarmWorkers,
    /// Let composed child commands own model sharing.
    DelegatedToSubcommands,
}

/// Whether the command benefits from cross-file or internal batching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BatchingPolicy {
    /// No profitable batching beyond ordinary per-file execution.
    None,
    /// Pool many files together into shared worker requests.
    CrossFileBatch,
    /// Keep the top-level unit per file, but allow internal stage batching.
    InternalStageBatching,
    /// One main file plus one paired reference artifact.
    PairedInputs,
}

/// How much per-command parallelism the shared kernel should expose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParallelismPolicy {
    /// Bound file-level concurrency and let the kernel auto-tune worker counts.
    BoundedFileWorkers,
    /// Keep one command-level dispatch at a time per job.
    SingleDispatchPerJob,
    /// Let composed child commands own their own parallelism.
    DelegatedToSubcommands,
}

/// How one command should behave on constrained-memory hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConstrainedHostPolicy {
    /// Allow the host to clamp execution to one worker and rely on lazy startup
    /// rather than speculative resident state.
    SequentialFallback,
    /// Let composed child commands own constrained-host behavior.
    DelegatedToSubcommands,
}

/// Whether the command should participate in optional background warmup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WarmupPolicy {
    /// The command should stay lazy/on-demand by default.
    LazyOnDemand,
    /// The host may warm this command in the background when capacity allows.
    BackgroundEligible,
    /// Let composed child commands own warmup behavior.
    DelegatedToSubcommands,
}

/// Dominant resource lane for the command's hot path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourceLane {
    /// GPU-backed workloads where device memory is the main bottleneck.
    GpuHeavy,
    /// CPU-bound workloads that still reuse warm model workers.
    CpuBound,
    /// Mostly IO / media feature extraction.
    IoBound,
    /// Mixed pipelines touching both CPU and GPU stages.
    Mixed,
}

/// Typed descriptor for one released command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CommandWorkflowDescriptor {
    /// Stable command name exposed to users.
    pub command: ReleasedCommand,
    /// Workflow family that owns the command semantics.
    pub family: WorkflowFamily,
    /// Primary infer task required by the worker layer.
    pub infer_task: InferTask,
    /// How the command is surfaced relative to the worker layer.
    pub capability_kind: CommandCapabilityKind,
    /// How the CLI ships inputs and whether paths mode is eligible.
    pub io_profile: CommandIoProfile,
    /// Which server-side runtime path currently owns this command.
    pub runner_dispatch_kind: RunnerDispatchKind,
}

/// Higher-level execution shape authored by commands.
///
/// This deliberately collapses the repeated low-level scheduling/profile knobs
/// into a smaller semantic vocabulary. Runtime-facing code derives its lower-
/// level policy directly from this shape instead of from a second authored
/// profile object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandExecutionShape {
    /// Cross-file text pooling with one top-level dispatch per job.
    BatchedText,
    /// Main transcript plus paired reference projection.
    ReferenceProjection,
    /// Audio-first sequential processing with bounded file-level concurrency.
    AudioSequential,
    /// Per-file media analysis over non-CHAT/audio feature inputs.
    MediaAnalysis,
    /// Composite command that delegates runtime policy to child flows.
    Composite,
}

impl CommandExecutionShape {
    /// High-level workflow family implied by this authored execution shape.
    pub(crate) const fn workflow_family(self) -> WorkflowFamily {
        match self {
            Self::BatchedText => WorkflowFamily::CrossFileBatchTransform,
            Self::ReferenceProjection => WorkflowFamily::ReferenceProjection,
            Self::AudioSequential | Self::MediaAnalysis => WorkflowFamily::PerFileTransform,
            Self::Composite => WorkflowFamily::Composite,
        }
    }

    /// High-level scheduling shape implied by this authored execution shape.
    pub(crate) const fn scheduling_policy(self) -> SchedulingPolicy {
        match self {
            Self::BatchedText => SchedulingPolicy::CrossFileBatch,
            Self::ReferenceProjection => SchedulingPolicy::ReferenceProjection,
            Self::AudioSequential => SchedulingPolicy::PerFileAudio,
            Self::MediaAnalysis => SchedulingPolicy::PerFileMediaAnalysis,
            Self::Composite => SchedulingPolicy::Composite,
        }
    }

    /// Model-sharing policy implied by this authored execution shape.
    pub(crate) const fn model_sharing_policy(self) -> ModelSharingPolicy {
        match self {
            Self::Composite => ModelSharingPolicy::DelegatedToSubcommands,
            Self::BatchedText
            | Self::ReferenceProjection
            | Self::AudioSequential
            | Self::MediaAnalysis => ModelSharingPolicy::SharedWarmWorkers,
        }
    }

    /// Batching policy implied by this authored execution shape.
    pub(crate) const fn batching_policy(self) -> BatchingPolicy {
        match self {
            Self::BatchedText => BatchingPolicy::CrossFileBatch,
            Self::ReferenceProjection => BatchingPolicy::PairedInputs,
            Self::AudioSequential => BatchingPolicy::InternalStageBatching,
            Self::MediaAnalysis | Self::Composite => BatchingPolicy::None,
        }
    }

    /// Parallelism policy implied by this authored execution shape.
    pub(crate) const fn parallelism_policy(self) -> ParallelismPolicy {
        match self {
            Self::AudioSequential | Self::MediaAnalysis => ParallelismPolicy::BoundedFileWorkers,
            Self::BatchedText | Self::ReferenceProjection => {
                ParallelismPolicy::SingleDispatchPerJob
            }
            Self::Composite => ParallelismPolicy::DelegatedToSubcommands,
        }
    }

    /// Dominant resource lane implied by this authored execution shape.
    pub(crate) const fn resource_lane(self) -> ResourceLane {
        match self {
            Self::BatchedText => ResourceLane::CpuBound,
            Self::ReferenceProjection | Self::Composite => ResourceLane::Mixed,
            Self::AudioSequential => ResourceLane::GpuHeavy,
            Self::MediaAnalysis => ResourceLane::IoBound,
        }
    }

    /// Constrained-host behavior implied by this authored execution shape.
    pub(crate) const fn constrained_host_policy(self) -> ConstrainedHostPolicy {
        match self {
            Self::Composite => ConstrainedHostPolicy::DelegatedToSubcommands,
            Self::BatchedText
            | Self::ReferenceProjection
            | Self::AudioSequential
            | Self::MediaAnalysis => ConstrainedHostPolicy::SequentialFallback,
        }
    }

    /// Warmup behavior implied by this authored execution shape.
    pub(crate) const fn warmup_policy(self) -> WarmupPolicy {
        match self {
            Self::MediaAnalysis => WarmupPolicy::LazyOnDemand,
            Self::Composite => WarmupPolicy::DelegatedToSubcommands,
            Self::BatchedText | Self::ReferenceProjection | Self::AudioSequential => {
                WarmupPolicy::BackgroundEligible
            }
        }
    }

    /// Whether host-memory admission should remain enabled for this shape.
    pub(crate) const fn uses_host_memory_gate(self) -> bool {
        true
    }
}

/// Canonical authored command definition.
///
/// Prefer the family authoring traits/macros below for ordinary command work.
/// These constructor helpers are the lower-level substrate those generated
/// declarations build on. Command modules should only hand-write a full
/// [`CommandWorkflowDescriptor`] when they are introducing a genuinely new
/// execution family or an unusual routing shape that the existing helpers do
/// not model yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CommandDefinition {
    /// Stable released-command descriptor.
    pub descriptor: CommandWorkflowDescriptor,
    /// Higher-level execution shape authored by the command.
    pub execution_shape: CommandExecutionShape,
}

impl CommandDefinition {
    /// High-level scheduling shape derived from the authored execution shape.
    pub(crate) const fn scheduling_policy(self) -> SchedulingPolicy {
        self.execution_shape.scheduling_policy()
    }

    /// Model-sharing policy derived from the authored execution shape.
    pub(crate) const fn model_sharing_policy(self) -> ModelSharingPolicy {
        self.execution_shape.model_sharing_policy()
    }

    /// Batching policy derived from the authored execution shape.
    pub(crate) const fn batching_policy(self) -> BatchingPolicy {
        self.execution_shape.batching_policy()
    }

    /// Parallelism policy derived from the authored execution shape.
    pub(crate) const fn parallelism_policy(self) -> ParallelismPolicy {
        self.execution_shape.parallelism_policy()
    }

    /// Dominant resource lane derived from the authored execution shape.
    pub(crate) const fn resource_lane(self) -> ResourceLane {
        self.execution_shape.resource_lane()
    }

    /// Constrained-host behavior derived from the authored execution shape.
    pub(crate) const fn constrained_host_policy(self) -> ConstrainedHostPolicy {
        self.execution_shape.constrained_host_policy()
    }

    /// Warmup behavior derived from the authored execution shape.
    pub(crate) const fn warmup_policy(self) -> WarmupPolicy {
        self.execution_shape.warmup_policy()
    }

    /// Whether host-memory admission should remain enabled.
    pub(crate) const fn uses_host_memory_gate(self) -> bool {
        self.execution_shape.uses_host_memory_gate()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BatchingPolicy, CommandExecutionShape, ConstrainedHostPolicy, SchedulingPolicy,
        WarmupPolicy,
    };

    #[test]
    fn batched_text_shape_keeps_cross_file_batch_contract() {
        let shape = CommandExecutionShape::BatchedText;
        assert_eq!(shape.scheduling_policy(), SchedulingPolicy::CrossFileBatch);
        assert_eq!(shape.batching_policy(), BatchingPolicy::CrossFileBatch);
        assert_eq!(
            shape.constrained_host_policy(),
            ConstrainedHostPolicy::SequentialFallback
        );
        assert_eq!(shape.warmup_policy(), WarmupPolicy::BackgroundEligible);
        assert!(shape.uses_host_memory_gate());
    }

    #[test]
    fn media_analysis_shape_stays_lazy_on_demand() {
        let shape = CommandExecutionShape::MediaAnalysis;
        assert_eq!(
            shape.scheduling_policy(),
            SchedulingPolicy::PerFileMediaAnalysis
        );
        assert_eq!(shape.batching_policy(), BatchingPolicy::None);
        assert_eq!(shape.warmup_policy(), WarmupPolicy::LazyOnDemand);
        assert!(shape.uses_host_memory_gate());
    }

    #[test]
    fn composite_shape_delegates_runtime_policy() {
        let shape = CommandExecutionShape::Composite;
        assert_eq!(shape.scheduling_policy(), SchedulingPolicy::Composite);
        assert_eq!(
            shape.constrained_host_policy(),
            ConstrainedHostPolicy::DelegatedToSubcommands
        );
        assert_eq!(shape.warmup_policy(), WarmupPolicy::DelegatedToSubcommands);
    }
}
