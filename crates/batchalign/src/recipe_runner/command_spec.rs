//! `CatalogEntry`: static metadata for one entry in the
//! `recipe_command_catalog()`.
//!
//! Renamed from `CommandSpec` in Phase β to free the `CommandSpec`
//! name for the public `batchalign-types::command_spec::CommandSpec`
//! (resource/classification spec). The two types are orthogonal,
//! cross-referenced by `ReleasedCommand` identity. See
//! `docs/architecture/2026-05-10-phase-beta-command-spec.md`.

use crate::api::ReleasedCommand;
use crate::worker::InferTask;

use super::materialize::OutputPolicy;
use super::recipe::{ExecutionMode, Recipe};

/// High-level command family in the replacement architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandFamily {
    /// Main transcript plus reference transcript projection.
    ReferenceProjection,
    /// Audio-first sequential recipes such as transcribe and align.
    AudioSequential,
    /// Cross-unit text commands that still expose per-file results.
    BatchedText,
    /// Composite commands that reuse other recipes.
    Composite,
    /// Media-analysis commands that emit non-CHAT artifacts.
    MediaAnalysis,
}

/// Which planner shape owns source discovery for a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlannerKind {
    /// Plain CHAT inputs with optional `--before`.
    TextInputs,
    /// Plain audio inputs.
    AudioInputs,
    /// Main transcript + gold companion pairing.
    ComparePairs,
    /// Audio input + derived gold CHAT pairing.
    BenchmarkPairs,
    /// Media-analysis audio inputs.
    MediaAnalysisInputs,
}

/// Whether a released command is owned directly by one recipe or by recipe composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapabilitySurface {
    /// One recipe owns the released command.
    RecipeOwned,
    /// The released command is defined by composing other recipes.
    Composite,
}

/// Worker-capability requirements for one released command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CapabilityPlan {
    /// Worker infer tasks required somewhere in the recipe.
    pub infer_tasks: &'static [InferTask],
    /// Whether the released command is recipe-owned or composed.
    pub surface: CapabilitySurface,
}

/// How one released command is surfaced relative to the worker infer-task layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandCapabilityKind {
    /// Command is advertised directly from one infer task.
    DirectInfer,
    /// Command is synthesized by Rust from lower-level infer capability.
    ServerComposed,
}

/// Which server-side runtime path currently owns one released command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunnerDispatchKind {
    /// Text-only commands pooled through the batched infer path.
    BatchedTextInfer,
    /// Forced alignment with per-file audio/media resolution.
    ForcedAlignment,
    /// Transcribe audio through the Rust-owned ASR orchestration path.
    TranscribeAudioInfer,
    /// Benchmark audio through the composite benchmark orchestrator.
    BenchmarkAudioInfer,
    /// Media-analysis V2 path for commands like openSMILE and AVQI.
    MediaAnalysisV2,
}

/// How the CLI should ship inputs to the server for this command.
///
/// Makes the content/paths-mode superset structural: a command either
/// uploads file bodies over HTTP, sends paths for CHAT-only inputs, or
/// sends paths plus requires shared-filesystem audio access. The illegal
/// combination "needs local audio but cannot use paths mode" is
/// unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandIoProfile {
    /// CLI uploads full file bodies over HTTP; server never reads client paths.
    ///
    /// Unconstructed today. Kept because it is the only way to express a
    /// command that cannot use paths mode at all, the correct shape for a
    /// REMOTE daemon (paths mode needs a shared filesystem). Its absence is
    /// why `supports_paths_mode()` is currently true for every released
    /// command.
    #[allow(dead_code, reason = "expresses remote-daemon-only commands; see above")]
    ContentOnly,
    /// CLI sends filesystem paths for text inputs; server reads CHAT directly.
    PathsModeText,
    /// CLI sends filesystem paths and the command also needs client-local
    /// audio on the shared filesystem (only valid for a local daemon).
    PathsModeAudio,
}

impl CommandIoProfile {
    /// Whether the server-side runner needs shared-filesystem audio access.
    pub const fn uses_local_audio(self) -> bool {
        matches!(self, Self::PathsModeAudio)
    }

    /// Whether the CLI may send paths instead of inlined content to a local daemon.
    pub const fn supports_paths_mode(self) -> bool {
        matches!(self, Self::PathsModeText | Self::PathsModeAudio)
    }
}

/// Static command metadata for the recipe-runner catalog.
///
/// Every field is DECLARED per command in `catalog.rs`; nothing here is
/// inferred from the command's name or position. Three of these fields
/// (`capability_kind`, `io_profile`, `runner_dispatch_kind`) were until
/// 2026-07-29 computed by `match` arms with a catch-all `_ =>` default, so a
/// newly released command silently inherited whatever the default happened to
/// be. Stating them here means a new entry cannot compile until each question
/// has an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CatalogEntry {
    /// Stable released command identity.
    pub command: ReleasedCommand,
    /// High-level family for contributor understanding.
    pub family: CommandFamily,
    /// Planner shape used to derive work units.
    pub planner: PlannerKind,
    /// Execution mode surfaced to the runtime.
    pub execution_mode: ExecutionMode,
    /// Whether the command is advertised straight from one infer task or
    /// synthesized by the server from lower-level capability.
    pub capability_kind: CommandCapabilityKind,
    /// How the CLI ships inputs and whether paths mode is eligible.
    pub io_profile: CommandIoProfile,
    /// Which server-side runtime path currently owns this command.
    pub runner_dispatch_kind: RunnerDispatchKind,
    /// Worker capability requirements.
    pub capabilities: CapabilityPlan,
    /// Output naming and sidecar policy.
    pub output_policy: OutputPolicy,
    /// Ordered stage recipe for the command.
    pub recipe: &'static Recipe,
}
