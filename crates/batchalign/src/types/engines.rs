//! Engine backend types and traits.
//!
//! Closed enum sets for ASR, FA, and UTR engine selection.
//! No external plugin system, all engines are built-in.
//! The [`EngineBackend`] trait provides a common interface.

use serde::{Deserialize, Serialize};

/// Shared behavior for all engine backend selectors.
///
/// Implement this on each engine enum so generic code can work across
/// engine categories without knowing which specific enum it holds.
pub trait EngineBackend: std::fmt::Debug + Clone + Send + Sync + 'static {
    /// Stable wire-format name used in JSON, CLI args, and SQLite.
    ///
    /// `&'static str`, not a borrow of `self`: every implementation returns a
    /// literal, and the narrower signature meant a category whose selection
    /// name IS its wire name could not say so without copying the table.
    fn wire_name(&self) -> &'static str;

    /// Whether this engine's inference is fully Rust-owned (no Python worker).
    fn is_rust_owned(&self) -> bool;

    /// Parse a wire-format name. Returns `None` for unrecognized names.
    fn try_from_wire_name(name: &str) -> Option<Self>
    where
        Self: Sized;
}

/// An engine category a user can choose from on the command line.
///
/// # Why this exists
///
/// There are four engine categories (ASR, UTR, FA, translate) and they had four
/// hand-written answers to the same five questions: which engines exist, what
/// is the default, what does a user type, what historical spellings still work,
/// and what does a name resolve to. Each answer was restated at the CLI, and
/// three of the four restatements had gone stale in the same direction: the
/// flag advertised a SUBSET, and the engines it left out were reachable only
/// through a second, differently-named flag taking an unvalidated string. That
/// is how the Cantonese engines stayed hidden from the users who needed them.
///
/// Fixing them one at a time did not work either: ASR was fixed first, and the
/// report that prompted this landed on UTR, one of the two still broken.
///
/// # What it buys
///
/// [`engine_selection_parser`] takes no free parameters. It previously took the
/// shown names, the hidden names, a resolver and a category string as four
/// unrelated arguments, which meant nothing stopped a caller pairing FA's names
/// with UTR's resolver and producing a flag whose help advertised three engines
/// it would then reject. Now the four facts travel together, from the type.
///
/// [`engine_selection_parser`]: crate::cli::args::commands
pub trait SelectableEngine: EngineBackend + Sized {
    /// What choosing a name yields.
    ///
    /// Usually `Self`. ASR is the exception: some of its names are an engine
    /// PLUS a checkpoint (`paraformer` is `funaudio` carrying `paraformer-zh`),
    /// so it selects an [`AsrSelection`] rather than a bare variant. Without
    /// this associated type the trait would either exclude ASR or force the
    /// other three into a wrapper they do not need.
    type Selected: Clone + Send + Sync + 'static;

    /// Every engine in this category, in help-display order.
    ///
    /// THE owner of "which engines exist" for the category.
    const ALL: &'static [Self];

    /// The engine used when the flag is omitted.
    const DEFAULT: Self;

    /// Human-readable category name, for diagnostics.
    ///
    /// What [`UnknownEngineName::category`] reports, via
    /// [`parse_wire_name`](Self::parse_wire_name), so each category string has
    /// one owner rather than one per error site.
    const CATEGORY: &'static str;

    /// The single name this engine is advertised under.
    ///
    /// Distinct from [`EngineBackend::wire_name`], which is persisted in JSON
    /// and SQLite and therefore cannot change. `--utr-engine tencent` beside
    /// `--asr-engine tencent` is one concept spelled one way, even though UTR's
    /// wire name is `tencent_utr`.
    fn selection_name(&self) -> &'static str;

    /// Every spelling accepted, canonical and historical, and what it means.
    ///
    /// COMPLETE: every canonical [`selection_name`](Self::selection_name)
    /// appears here, followed by that engine's historical spellings. The first
    /// draft of this trait let the table mean something different per category
    /// (UTR listed only wire names, FA listed some canonical names and not
    /// others), which forced three different resolvers and quietly weakened
    /// the coherence test to two of four categories.
    ///
    /// One table per category, read by the resolver, by
    /// [`try_from_wire_name`](EngineBackend::try_from_wire_name) and by the
    /// CLI's hidden list, so a spelling cannot be accepted by one and rejected
    /// by another. Clap rejects anything outside its list BEFORE the resolver
    /// runs, so a hidden list that falls short of the resolver silently breaks
    /// old command lines.
    fn accepted_names() -> &'static [(&'static str, Self)];

    /// Resolve a user-typed name to a variant.
    ///
    /// Provided, because with a complete table there is one answer. Three
    /// hand-written bodies preceded this, two of which did the same two lookups
    /// in opposite orders.
    fn resolve_variant(name: &str) -> Option<Self> {
        Self::accepted_names()
            .iter()
            .find(|(accepted, _)| *accepted == name)
            .map(|(_, engine)| engine.clone())
    }

    /// Resolve a user-typed name to whatever this category selects.
    ///
    /// Defaults to [`resolve_variant`](Self::resolve_variant). ASR overrides
    /// it, because a name there can carry a checkpoint as well as an engine.
    fn resolve(name: &str) -> Option<Self::Selected>;

    /// The names shown in `--help`.
    ///
    /// Overridable: ASR appends `paraformer`, which names a selection rather
    /// than a variant.
    fn selectable_names() -> impl Iterator<Item = &'static str> {
        Self::ALL.iter().map(Self::selection_name)
    }

    /// The names accepted but not advertised: everything that is not the one
    /// canonical selection name for its engine.
    ///
    /// Derived, so the hidden set cannot fall short of the resolver.
    fn hidden_alias_names() -> impl Iterator<Item = &'static str> {
        Self::accepted_names()
            .iter()
            .filter(|(name, engine)| *name != engine.selection_name())
            .map(|(name, _)| *name)
    }

    /// Parse a persisted wire-format token, reporting the category on failure.
    ///
    /// Each category had its own copy of this body and its own copy of the
    /// category literal, four of each. The literal now comes from
    /// [`CATEGORY`](Self::CATEGORY).
    fn parse_wire_name(name: &str) -> Result<Self, UnknownEngineName> {
        Self::try_from_wire_name(name).ok_or_else(|| UnknownEngineName {
            name: name.to_owned(),
            category: Self::CATEGORY,
        })
    }
}

impl SelectableEngine for AsrEngineName {
    /// ASR selects an [`AsrSelection`], not a bare variant: `paraformer` is
    /// `funaudio` plus a checkpoint, so a name is not always an engine.
    type Selected = AsrSelection;
    const ALL: &'static [Self] = &[
        Self::RevAi,
        Self::Whisper,
        Self::WhisperHub,
        Self::WhisperX,
        Self::WhisperOai,
        Self::WhisperRs,
        Self::HkTencent,
        Self::HkAliyun,
        Self::HkFunaudio,
        Self::HkQwen,
    ];
    const DEFAULT: Self = Self::RevAi;
    const CATEGORY: &'static str = "ASR";

    fn selection_name(&self) -> &'static str {
        // Identical to the wire name for this category, so there is one table.
        self.wire_name_const()
    }

    fn accepted_names() -> &'static [(&'static str, Self)] {
        Self::ACCEPTED_NAMES
    }

    fn resolve(name: &str) -> Option<AsrSelection> {
        AsrSelection::parse(name)
    }

    /// Overridden to append `paraformer`, which names a SELECTION rather than
    /// a variant and so cannot come from `ALL`.
    fn selectable_names() -> impl Iterator<Item = &'static str> {
        Self::ALL
            .iter()
            .map(|engine| engine.wire_name_const())
            .chain(std::iter::once(PARAFORMER_SELECTION_NAME))
    }
}

impl AsrEngineName {
    /// Every spelling accepted. One historical entry: `whisper-oai` was the
    /// CLI's spelling of the `whisper_oai` wire name, which deriving the value
    /// list from the enum immediately exposed as two public names for one
    /// engine.
    const ACCEPTED_NAMES: &'static [(&'static str, Self)] = &[
        ("rev", Self::RevAi),
        ("whisper", Self::Whisper),
        ("whisper_hub", Self::WhisperHub),
        ("whisperx", Self::WhisperX),
        ("whisper_oai", Self::WhisperOai),
        ("whisper-oai", Self::WhisperOai),
        ("whisper_rs", Self::WhisperRs),
        ("tencent", Self::HkTencent),
        ("aliyun", Self::HkAliyun),
        ("funaudio", Self::HkFunaudio),
        ("qwen", Self::HkQwen),
    ];
}

impl SelectableEngine for TranslateEngineName {
    type Selected = Self;
    const ALL: &'static [Self] = &[
        Self::Google,
        Self::Seamless,
        Self::Nllb,
        Self::Tencent,
        Self::Aliyun,
    ];
    const DEFAULT: Self = Self::Google;
    const CATEGORY: &'static str = "translate";

    fn selection_name(&self) -> &'static str {
        // Identical to the wire name for this category, so there is one table.
        self.wire_name()
    }

    fn accepted_names() -> &'static [(&'static str, Self)] {
        Self::ACCEPTED_NAMES
    }

    fn resolve(name: &str) -> Option<Self> {
        Self::resolve_variant(name)
    }
}

impl TranslateEngineName {
    /// Every spelling accepted. No historical aliases: the CLI names and the
    /// wire names were already identical, which is why this category's
    /// duplicate CLI enum stayed silently in agreement rather than drifting
    /// like the other three.
    const ACCEPTED_NAMES: &'static [(&'static str, Self)] = &[
        ("google", Self::Google),
        ("seamless", Self::Seamless),
        ("nllb", Self::Nllb),
        ("tencent", Self::Tencent),
        ("aliyun", Self::Aliyun),
    ];
}

/// Error returned when a wire-format engine name is not recognized.
#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown engine name \"{name}\" for {category}")]
pub struct UnknownEngineName {
    /// The unrecognized wire name.
    pub name: String,
    /// Which engine category was being parsed (e.g. "ASR", "FA", "UTR").
    pub category: &'static str,
}

/// Typed UTR engine selector.
///
/// The wire format still uses the legacy string tokens (`"rev_utr"`,
/// `"whisper_utr"`, or a plugin-provided name), but the server runtime works
/// with this enum so the control plane stops branching on anonymous strings.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UtrEngine {
    /// Rust-owned Rev.AI timed-word path.
    RevAi,
    /// Python-worker ASR path with the built-in Whisper profile.
    Whisper,
    /// Tencent UTR (HK/Cantonese).
    HkTencent,
}

impl EngineBackend for UtrEngine {
    fn wire_name(&self) -> &'static str {
        match self {
            Self::RevAi => "rev_utr",
            Self::Whisper => "whisper_utr",
            Self::HkTencent => "tencent_utr",
        }
    }

    fn is_rust_owned(&self) -> bool {
        matches!(self, Self::RevAi)
    }

    fn try_from_wire_name(name: &str) -> Option<Self> {
        // The one table, so a spelling cannot be accepted here and rejected by
        // the CLI resolver, or the reverse.
        Self::resolve_variant(name)
    }
}

impl SelectableEngine for UtrEngine {
    type Selected = Self;
    const ALL: &'static [Self] = &[Self::RevAi, Self::Whisper, Self::HkTencent];
    const DEFAULT: Self = Self::RevAi;
    const CATEGORY: &'static str = "UTR";

    fn selection_name(&self) -> &'static str {
        match self {
            Self::RevAi => "rev",
            Self::Whisper => "whisper",
            Self::HkTencent => "tencent",
        }
    }

    fn accepted_names() -> &'static [(&'static str, Self)] {
        Self::ACCEPTED_NAMES
    }

    fn resolve(name: &str) -> Option<Self> {
        Self::resolve_variant(name)
    }
}

impl UtrEngine {
    /// Canonical names first, then the wire names, which are the historical
    /// half: persisted in JSON and SQLite so they cannot change, but also not
    /// what a user should have to type.
    const ACCEPTED_NAMES: &'static [(&'static str, Self)] = &[
        ("rev", Self::RevAi),
        ("whisper", Self::Whisper),
        ("tencent", Self::HkTencent),
        ("rev_utr", Self::RevAi),
        ("whisper_utr", Self::Whisper),
        ("tencent_utr", Self::HkTencent),
    ];

    /// Parse one persisted wire-format token.
    ///
    /// Forwards to [`SelectableEngine::parse_wire_name`], which owns both
    /// this body and the category name. All four categories had their own
    /// copy of each.
    pub fn from_wire_name(name: &str) -> Result<Self, UnknownEngineName> {
        <Self as SelectableEngine>::parse_wire_name(name)
    }

    /// Borrow the wire-format token for JSON/SQLite.
    pub fn as_wire_name(&self) -> &str {
        self.wire_name()
    }

    /// Whether the current engine can reuse the worker-side segment strategy
    /// for partial-window UTR.
    pub fn supports_partial_windows(&self) -> bool {
        !self.is_rust_owned()
    }
}

impl Serialize for UtrEngine {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_wire_name())
    }
}

impl<'de> Deserialize<'de> for UtrEngine {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Self::from_wire_name(&name).map_err(serde::de::Error::custom)
    }
}

/// Typed forced-alignment engine selector.
///
/// The wire format still uses the legacy string tokens (`"wav2vec_fa"`,
/// `"whisper_fa"`, or a plugin-provided name), but the control plane works
/// with this enum so dispatch does not branch on anonymous strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FaEngineName {
    /// MMS Wave2Vec forced alignment.
    Wave2Vec,
    /// Whisper token-timestamp forced alignment.
    Whisper,
    /// Wav2Vec Cantonese forced alignment (HK).
    Wav2vecCanto,
}

/// What an alignment engine can say about a word's extent.
///
/// The distinction decides whether a `%wor` tier carries measured durations or
/// derived ones, so it belongs to the engine rather than to any consumer. It
/// had previously been restated in prose in several places, in one case
/// inverted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaTimingResolution {
    /// Reports a word's start AND its end; durations are measured.
    WordIntervals,
    /// Reports only when a word STARTS; an end must be derived from the next
    /// onset, and a consumer that skips that step produces zero-duration words.
    TokenOnsets,
}

impl FaEngineName {
    /// What this engine reports about a word's extent.
    pub fn timing_resolution(&self) -> FaTimingResolution {
        match self {
            Self::Wave2Vec | Self::Wav2vecCanto => FaTimingResolution::WordIntervals,
            Self::Whisper => FaTimingResolution::TokenOnsets,
        }
    }

    /// Longest audio window handed to this engine in one dispatch.
    ///
    /// A per-engine fact, so it lives on the engine rather than in a match at
    /// the dispatch site: a new engine then cannot be added without stating
    /// its window.
    pub fn max_group_ms(&self) -> batchalign_types::domain::DurationMs {
        match self {
            // The CTC decoder's target length grows with the audio window, so
            // wav2vec takes the shorter one.
            Self::Wave2Vec | Self::Wav2vecCanto => batchalign_types::domain::DurationMs(15_000),
            Self::Whisper => batchalign_types::domain::DurationMs(20_000),
        }
    }
}

impl EngineBackend for FaEngineName {
    fn wire_name(&self) -> &'static str {
        match self {
            Self::Wave2Vec => "wav2vec_fa",
            Self::Whisper => "whisper_fa",
            Self::Wav2vecCanto => "cantonese_fa",
        }
    }

    fn is_rust_owned(&self) -> bool {
        false
    }

    fn try_from_wire_name(name: &str) -> Option<Self> {
        Self::ACCEPTED_NAMES
            .iter()
            .find(|(accepted, _)| *accepted == name)
            .map(|(_, engine)| *engine)
    }
}

impl SelectableEngine for FaEngineName {
    type Selected = Self;
    const ALL: &'static [Self] = &[Self::Wave2Vec, Self::Whisper, Self::Wav2vecCanto];
    // Wave2Vec returns word-level start AND end; Whisper FA returns token
    // onsets only, so an end has to be derived from the next onset and the
    // last word of a group has none to derive from. Measured beats derived,
    // which is why the default is the engine that measures. Pinned by
    // `default_fa_engine_reports_word_intervals`, which asserts the PROPERTY
    // rather than the variant.
    const DEFAULT: Self = Self::Wave2Vec;
    const CATEGORY: &'static str = "FA";

    fn selection_name(&self) -> &'static str {
        match self {
            Self::Wave2Vec => "wav2vec",
            Self::Whisper => "whisper",
            Self::Wav2vecCanto => "cantonese",
        }
    }

    fn accepted_names() -> &'static [(&'static str, Self)] {
        Self::ACCEPTED_NAMES
    }

    fn resolve(name: &str) -> Option<Self> {
        Self::resolve_variant(name)
    }
}

impl FaEngineName {
    /// Canonical names first, then every historical spelling. This category had
    /// the most in circulation: the Cantonese engine alone answered to three,
    /// and the book used a fourth.
    const ACCEPTED_NAMES: &'static [(&'static str, Self)] = &[
        ("wav2vec", Self::Wave2Vec),
        ("whisper", Self::Whisper),
        ("cantonese", Self::Wav2vecCanto),
        ("wav2vec_fa", Self::Wave2Vec),
        ("wave2vec", Self::Wave2Vec),
        ("whisper_fa", Self::Whisper),
        ("cantonese_fa", Self::Wav2vecCanto),
        ("wav2vec_canto", Self::Wav2vecCanto),
        ("wav2vec_fa_canto", Self::Wav2vecCanto),
    ];

    /// The override name used in worker pool keys for dispatch.
    ///
    /// Must match `fa_backend_override_name()` in `worker/pool/execute_v2.rs`.
    /// These are the names the Python worker sees in its `--engine-overrides`
    /// JSON and uses to select which FA model to load.
    pub fn dispatch_override_name(&self) -> &'static str {
        match self {
            Self::Wave2Vec => "wave2vec",
            Self::Whisper => "whisper",
            Self::Wav2vecCanto => "wav2vec_canto",
        }
    }

    /// Parse one persisted wire-format token.
    ///
    /// Forwards to [`SelectableEngine::parse_wire_name`], which owns both
    /// this body and the category name. All four categories had their own
    /// copy of each.
    pub fn from_wire_name(name: &str) -> Result<Self, UnknownEngineName> {
        <Self as SelectableEngine>::parse_wire_name(name)
    }

    /// Borrow the wire-format token for JSON/SQLite.
    pub fn as_wire_name(&self) -> &str {
        self.wire_name()
    }

    /// Resident memory footprint estimate for one worker process running
    /// this FA engine, in MB. Used by the admission gate to reserve enough
    /// headroom for engines whose actual RSS exceeds the default GPU-profile
    /// reservation (``tier.gpu_startup_mb``: 6 GB Small / 3 GB Medium /
    /// 16 GB Large+Fleet). See
    /// [`super::super::worker::pool::memory_gate::engine_aware_startup_reservation_mb`].
    pub fn resident_memory_mb(&self) -> u64 {
        match self {
            // Whisper-large-v2 FA: ~3 GB weights + tokenizer + Python
            // runtime. Same shape as Whisper-large-v3 ASR, hence the
            // shared constant.
            Self::Whisper => WHISPER_LARGE_V3_RSS_MB,
            // MMS / torchaudio Wave2Vec FA models: ~1.2 GB + runtime
            // margin. Cantonese FA is the same shape.
            Self::Wave2Vec | Self::Wav2vecCanto => WAVE2VEC_FA_RSS_MB,
        }
    }
}

impl Serialize for FaEngineName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_wire_name())
    }
}

impl<'de> Deserialize<'de> for FaEngineName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Self::from_wire_name(&name).map_err(serde::de::Error::custom)
    }
}

/// Typed ASR engine selector.
///
/// The wire format still uses the legacy string tokens (`"rev"`,
/// `"whisper"`, `"whisperx"`, `"whisper_oai"`, or a plugin-provided name), but
/// the control plane works with this enum so backend selection is explicit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AsrEngineName {
    /// Rust-owned Rev.AI backend.
    RevAi,
    /// Local Whisper worker backend.
    Whisper,
    /// HuggingFace Whisper fine-tune backend. Loads community fine-tunes
    /// by model_id (resolved per-language, with an explicit override in
    /// ``engine_overrides.model_id``). See
    /// ``book/src/batchalign/reference/whisper-hub-asr.md``.
    WhisperHub,
    /// WhisperX worker backend.
    WhisperX,
    /// OpenAI Whisper API backend.
    WhisperOai,
    /// Rust-native Whisper backend (whisper.cpp via whisper-rs), run
    /// in-process instead of through the Python worker. Rust-owned; the
    /// `whisper-rs-backend` Cargo feature is DEFAULT since 2026-07-28, the
    /// model auto-resolves (``BATCHALIGN_WHISPER_RS_MODEL`` override, else
    /// ggml-large-v3 fetched once via hf-hub), and language `Auto` engages
    /// whisper.cpp's own detection. See ``book/src/batchalign/reference/whisper-asr.md``.
    WhisperRs,
    /// Tencent Cloud ASR (HK/Cantonese).
    HkTencent,
    /// Aliyun ASR (HK/Cantonese).
    HkAliyun,
    /// FunAudio ASR (HK/Cantonese).
    HkFunaudio,
    /// Qwen3-ASR (Alibaba, HK/Cantonese). Local model loaded via the
    /// ``qwen-asr`` Python package. Open-weight Cantonese-capable ASR;
    /// external evaluations report competitive CER on per-utterance
    /// child speech.
    HkQwen,
}

impl AsrEngineName {
    /// The wire name, usable in a `const` context.
    ///
    /// THE owner of the table; [`EngineBackend::wire_name`] delegates here. It
    /// is inherent and `const` so a constant can be DERIVED from a variant
    /// rather than restating its spelling: `DEFAULT_ASR_SELECTION_NAME` was
    /// the literal `"rev"` written beside `AsrEngineName::RevAi`, and nothing
    /// tied the two together.
    pub const fn wire_name_const(&self) -> &'static str {
        match self {
            Self::RevAi => "rev",
            Self::Whisper => "whisper",
            Self::WhisperHub => "whisper_hub",
            Self::WhisperX => "whisperx",
            Self::WhisperOai => "whisper_oai",
            Self::WhisperRs => "whisper_rs",
            Self::HkTencent => "tencent",
            Self::HkAliyun => "aliyun",
            Self::HkFunaudio => "funaudio",
            Self::HkQwen => "qwen",
        }
    }
}

impl EngineBackend for AsrEngineName {
    fn wire_name(&self) -> &'static str {
        self.wire_name_const()
    }

    fn is_rust_owned(&self) -> bool {
        matches!(self, Self::RevAi | Self::WhisperRs)
    }

    fn try_from_wire_name(name: &str) -> Option<Self> {
        match name {
            "rev" => Some(Self::RevAi),
            "whisper" => Some(Self::Whisper),
            "whisper_hub" => Some(Self::WhisperHub),
            "whisperx" => Some(Self::WhisperX),
            "whisper_oai" => Some(Self::WhisperOai),
            "whisper_rs" => Some(Self::WhisperRs),
            "tencent" => Some(Self::HkTencent),
            "aliyun" => Some(Self::HkAliyun),
            "funaudio" => Some(Self::HkFunaudio),
            "qwen" => Some(Self::HkQwen),
            _ => None,
        }
    }
}

impl AsrEngineName {
    // `ALL` and `selectable_names` used to live here as inherent items, and
    // this diff added them AGAIN as part of the `SelectableEngine` impl. Two
    // owners of one list, which is the defect the trait exists to remove, and
    // the worse half is that inherent items WIN method resolution over trait
    // items: `AsrEngineName::ALL` silently meant the inherent copy, so the two
    // could drift with nothing to notice. The trait impl is the only one now.

    /// The override name used in worker pool keys for dispatch, or `None` for
    /// cloud-only engines (Rev.AI) that don't need a local worker.
    ///
    /// `execute_v2` reaches this through `EngineSelection` rather than keeping
    /// its own copy, so there is no second table to keep in step.
    pub fn dispatch_override_name(&self) -> Option<&'static str> {
        match self {
            Self::Whisper => Some("whisper"),
            Self::WhisperHub => Some("whisper_hub"),
            Self::HkTencent => Some("tencent"),
            Self::HkAliyun => Some("aliyun"),
            Self::HkFunaudio => Some("funaudio"),
            Self::HkQwen => Some("qwen"),
            // Rust-owned in-process paths (no pool-managed Python worker):
            // Rev.AI and WhisperRs; plus the cloud HTTP engines.
            Self::RevAi | Self::WhisperX | Self::WhisperOai | Self::WhisperRs => None,
        }
    }

    /// Parse one persisted wire-format token.
    ///
    /// Forwards to [`SelectableEngine::parse_wire_name`], which owns both
    /// this body and the category name. All four categories had their own
    /// copy of each.
    pub fn from_wire_name(name: &str) -> Result<Self, UnknownEngineName> {
        <Self as SelectableEngine>::parse_wire_name(name)
    }

    /// Borrow the wire-format token for JSON/SQLite.
    pub fn as_wire_name(&self) -> &str {
        self.wire_name()
    }

    /// Resident memory footprint estimate for one worker process running
    /// this ASR engine, in MB. Used by the admission gate to reserve
    /// enough headroom for engines whose actual RSS exceeds the default
    /// GPU-profile reservation (``tier.gpu_startup_mb``: 6 GB Small /
    /// 3 GB Medium / 16 GB Large+Fleet). See
    /// [`super::super::worker::pool::memory_gate::engine_aware_startup_reservation_mb`].
    pub fn resident_memory_mb(&self) -> u64 {
        match self {
            // Whisper-large-v3 (and its WhisperHub fine-tunes): ~3 GB
            // model + tokenizer + Python runtime. WhisperX is included
            // here for symmetry/future-proofing even though
            // ``dispatch_override_name`` returns ``None`` for it today
            // (it doesn't get a pool-managed Python worker), so the
            // admission gate never observes this value in production.
            Self::Whisper | Self::WhisperHub | Self::WhisperX => WHISPER_LARGE_V3_RSS_MB,
            // whisper.cpp large-v3 loaded in-process (Rust). Same RSS class as
            // the Python Whisper worker. Runs in the main process (no pool
            // worker), so the worker-admission gate never observes this value;
            // classified here for symmetry.
            Self::WhisperRs => WHISPER_LARGE_V3_RSS_MB,
            // Local model: Qwen3-ASR-1.7B weights (~3.4 GB fp16 /
            // ~7 GB fp32) + tokenizer + Python runtime. Same RSS
            // class as Whisper-large-v3; pinned via the
            // ``asr_engine_qwen_resident_memory_matches_local_model_footprint``
            // test in this module.
            Self::HkQwen => WHISPER_LARGE_V3_RSS_MB,
            // Cloud HTTP clients with no local model. FunASR is
            // grouped here for historical reasons even though
            // SenseVoiceSmall is a local model; the wrapper's
            // resident footprint is closer to a cloud client because
            // it offloads to ModelScope's cached model server.
            // Re-classify if a long-form FunASR run on a tight host
            // ever OOM-kills.
            Self::RevAi
            | Self::WhisperOai
            | Self::HkTencent
            | Self::HkAliyun
            | Self::HkFunaudio => HTTP_CLIENT_BASELINE_RSS_MB,
        }
    }

    /// Whether this engine is the Rust-owned Rev.AI path.
    pub fn is_revai(&self) -> bool {
        matches!(self, Self::RevAi)
    }
}

impl Serialize for AsrEngineName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_wire_name())
    }
}

impl<'de> Deserialize<'de> for AsrEngineName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Self::from_wire_name(&name).map_err(serde::de::Error::custom)
    }
}

/// Typed translation engine selector.
///
/// The wire format uses the lowercase tokens ``"google"``,
/// ``"seamless"``, ``"nllb"``, ``"tencent"``, and ``"aliyun"``; the
/// Python worker's ``resolve_translate_engine``
/// (``batchalign/worker/_model_loading/translation.py``) matches on
/// those exact strings. Any change here must be mirrored on the Python
/// side or dispatch breaks silently.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TranslateEngineName {
    /// Public Google Translate via the ``googletrans`` library. Requires
    /// outbound reachability to ``translate.google.com``, unsuitable
    /// behind the Great Firewall without a VPN.
    Google,
    /// Local Meta SeamlessM4T model, loaded from HuggingFace and run
    /// in-process in the Python worker. No outbound network at
    /// inference time. Retained for back-compat with BA2 callers;
    /// short-CJK quality is poor, prefer ``Nllb`` or ``Tencent`` for
    /// new work.
    Seamless,
    /// Local Meta NLLB-200-distilled-1.3B (~5 GB), text-MT-native.
    /// No outbound network at inference time. Self-hosted fallback
    /// that handles Cantonese first-class (Tencent does not).
    Nllb,
    /// Tencent Cloud TMT (Text Translation), cloud-API engine.
    /// Strong quality on Mandarin (``zh→en``); does NOT support
    /// Cantonese (``yue``). Requires CAM credentials with
    /// ``tmt:TextTranslate`` permission in ``~/.batchalign.ini``
    /// or via ``BATCHALIGN_TENCENT_{ID,KEY,REGION}`` environment
    /// variables. Free tier 5M chars/month.
    Tencent,
    /// Aliyun (Alibaba Cloud) Machine Translation, cloud-API engine.
    /// Supports Cantonese (``yue``) as a source language, which Tencent
    /// TMT does not: the canonical cloud translate option for HK
    /// Cantonese material. Requires access-key credentials in
    /// ``~/.batchalign.ini`` ``[asr]`` section
    /// (``engine.aliyun.id``/``key``/``region``, shared with the Aliyun
    /// ASR backend) or via ``BATCHALIGN_ALIYUN_{ID,KEY,REGION}``
    /// environment variables. Quotas and pricing per Aliyun MT service
    /// terms.
    Aliyun,
}

impl EngineBackend for TranslateEngineName {
    fn wire_name(&self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Seamless => "seamless",
            Self::Nllb => "nllb",
            Self::Tencent => "tencent",
            Self::Aliyun => "aliyun",
        }
    }

    fn is_rust_owned(&self) -> bool {
        // All backends run in the Python worker. No Rust-owned
        // translate path exists today.
        false
    }

    fn try_from_wire_name(name: &str) -> Option<Self> {
        // The one table, so a spelling cannot be accepted here and rejected by
        // the CLI resolver, or the reverse.
        Self::resolve_variant(name)
    }
}

impl TranslateEngineName {
    /// The override name used in worker pool keys for dispatch.
    ///
    /// Identical to ``wire_name``: translate has no legacy alias
    /// divergence between dispatch and wire today. Provided for
    /// shape-parity with ``AsrEngineName`` and ``FaEngineName``.
    pub fn dispatch_override_name(&self) -> &'static str {
        // Identical to the wire name, which its previous doc admitted; a third
        // copy of one five-string table is three places to mistype it.
        self.wire_name()
    }

    /// Parse one persisted wire-format token.
    ///
    /// Forwards to [`SelectableEngine::parse_wire_name`], which owns both
    /// this body and the category name. All four categories had their own
    /// copy of each.
    pub fn from_wire_name(name: &str) -> Result<Self, UnknownEngineName> {
        <Self as SelectableEngine>::parse_wire_name(name)
    }

    /// Borrow the wire-format token for JSON/SQLite.
    pub fn as_wire_name(&self) -> &str {
        self.wire_name()
    }

    /// Resident memory footprint estimate for one worker process
    /// running this translate engine, in MB. Used by the admission
    /// gate to reserve enough headroom for engines whose actual RSS
    /// exceeds the default IO-profile reservation
    /// (``tier.io_startup_mb``: 2 GB Small/Medium, 4 GB Large/Fleet).
    /// The estimate is the observed model + tokenizer + Python
    /// runtime footprint with a modest margin; conservative on the
    /// side of over-reserving so the OS OOM killer isn't the fallback
    /// safety mechanism. Related but distinct from the *on-disk*
    /// model-size hints used by the Python progress events
    /// (``batchalign/worker/_progress.py::_HF_SIZE_HINTS_GB``).
    pub fn resident_memory_mb(&self) -> u64 {
        match self {
            // googletrans + Tencent TMT + Aliyun MT are all thin
            // HTTP-client engines with no local model loaded, same
            // baseline. The Aliyun MT REST client and ``googletrans``
            // both wrap ``requests``/``aiohttp``-style transports;
            // there is no per-process model state to account for.
            Self::Google | Self::Tencent | Self::Aliyun => HTTP_CLIENT_BASELINE_RSS_MB,
            Self::Seamless => SEAMLESS_M4T_MEDIUM_RSS_MB,
            Self::Nllb => NLLB_200_DISTILLED_1_3B_RSS_MB,
        }
    }
}

/// Resident memory estimate for any worker that runs a thin HTTP-client
/// engine with no local model loaded, googletrans for translate, and
/// the cloud ASR engines (Rev.AI, WhisperOai, HkTencent, HkAliyun,
/// HkFunaudio). Baseline Python + worker scaffolding only.
pub(crate) const HTTP_CLIENT_BASELINE_RSS_MB: u64 = 200;

/// Resident memory estimate for a worker running the local
/// SeamlessM4T-medium model: ~2.4 GB weights + tokenizer + runtime,
/// with margin.
pub(crate) const SEAMLESS_M4T_MEDIUM_RSS_MB: u64 = 2_900;

/// Resident memory estimate for a worker running the local
/// NLLB-200-distilled-1.3B model: ~5 GB weights + tokenizer +
/// runtime, with margin.
pub(crate) const NLLB_200_DISTILLED_1_3B_RSS_MB: u64 = 5_500;

/// Resident memory estimate for a worker running the Whisper-large-v3
/// ASR model or the Whisper-large-v2 FA model (same shape). ~3 GB
/// weights + tokenizer + Python runtime + margin.
pub(crate) const WHISPER_LARGE_V3_RSS_MB: u64 = 3_500;

/// Resident memory estimate for a worker running an MMS / Wave2Vec
/// forced-alignment model (including the Cantonese variant): ~1.2 GB
/// torchaudio weights + runtime margin.
pub(crate) const WAVE2VEC_FA_RSS_MB: u64 = 1_800;

impl Serialize for TranslateEngineName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_wire_name())
    }
}

impl<'de> Deserialize<'de> for TranslateEngineName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Self::from_wire_name(&name).map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// EngineOverrides: typed engine override selection
// ---------------------------------------------------------------------------

/// Typed engine overrides for one job or worker spawn.
///
/// Replaces `BTreeMap<String, String>` in `CommonOptions.engine_overrides`.
/// Only populated fields are serialized; empty overrides produce `{}`.
///
/// Three top-level fields are typed (``asr`` / ``fa`` / ``translate``)
/// because they pick *which* engine runs. Any other key is preserved
/// as an opaque per-engine configuration extra in [`Self::extras`].
/// This is how the Python worker receives per-engine knobs such as
/// ``qwen_model``, ``qwen_device``, ``funaudio_*``, etc., adding a
/// new engine knob does NOT require a Rust schema change, but a typo
/// in a knob name will reach Python where the engine loader chooses
/// whether to use a default or error. (A future engine registry
/// task #66 / Phase 5c, replaces this string-keyed map with typed
/// per-engine payload structs.)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct EngineOverrides {
    /// ASR engine override (e.g., `AsrEngineName::HkTencent`).
    pub asr: Option<AsrEngineName>,
    /// FA engine override (e.g., `FaEngineName::Wav2vecCanto`).
    pub fa: Option<FaEngineName>,
    /// UTR engine override (e.g., `UtrEngine::HkTencent`).
    ///
    /// Added late, and its absence was a silent-discard bug rather than a
    /// deliberate omission: `utr` is not one of the typed keys, so
    /// `--engine-overrides '{"utr":"whisper_utr"}'` fell through to
    /// [`Self::extras`] and was shipped to a Python worker that has no say in
    /// UTR engine selection at all. The user's choice vanished with no message.
    pub utr: Option<UtrEngine>,
    /// Translate engine override (e.g., `TranslateEngineName::Seamless`).
    pub translate: Option<TranslateEngineName>,
    /// Opaque per-engine configuration knobs (e.g., ``qwen_model``,
    /// ``qwen_device``). Round-trips verbatim through the JSON
    /// boundary so the Python worker bootstrap can read them by name.
    pub extras: std::collections::BTreeMap<String, String>,
}

impl EngineOverrides {
    /// Return `true` when no overrides are set.
    pub fn is_empty(&self) -> bool {
        self.utr.is_none()
            && self.asr.is_none()
            && self.fa.is_none()
            && self.translate.is_none()
            && self.extras.is_empty()
    }

    /// Serialize to a JSON string in the PERSISTENCE wire format
    /// (`wire_name()` tokens). For anything that reaches a worker
    /// (pool keys, capability-discovery spawns, worker argv), use
    /// [`Self::to_dispatch_json_string`] instead.
    ///
    /// Returns empty string when no overrides are set.
    pub fn to_json_string(&self) -> String {
        if self.is_empty() {
            String::new()
        } else {
            serde_json::to_string(self).unwrap_or_else(|e| format!("<serialization failed: {e}>"))
        }
    }

    /// Produce the worker-facing override map, using the DISPATCH names the
    /// Python engine loaders accept (`dispatch_override_name()`), NOT the
    /// persistence wire names (`wire_name()`).
    ///
    /// The two schemes differ for every FA engine ("wav2vec_fa" /
    /// "whisper_fa" / "cantonese_fa" persisted vs "wave2vec" /
    /// "whisper" / "wav2vec_canto" dispatched). Sending a persistence
    /// name kills the worker at bootstrap: `resolve_fa_engine` raises
    /// before the ready signal, which failed four consecutive align
    /// jobs on a fleet host on 2026-06-11.
    ///
    /// Cloud-only ASR engines with no local worker (Rev.AI, WhisperX,
    /// WhisperOai) have no dispatch name and are omitted. Extras
    /// round-trip verbatim, exactly as in [`Self::to_json_string`]
    /// (the 2026-05-27 `qwen_model` lesson).
    ///
    pub fn dispatch_overrides(&self) -> std::collections::BTreeMap<String, String> {
        let mut map = std::collections::BTreeMap::new();
        if let Some(ref asr) = self.asr
            && let Some(name) = asr.dispatch_override_name()
        {
            map.insert("asr".to_owned(), name.to_owned());
        }
        if let Some(ref fa) = self.fa {
            map.insert("fa".to_owned(), fa.dispatch_override_name().to_owned());
        }
        if let Some(ref translate) = self.translate {
            map.insert(
                "translate".to_owned(),
                translate.dispatch_override_name().to_owned(),
            );
        }
        for (key, value) in &self.extras {
            map.insert(key.clone(), value.clone());
        }
        map
    }

    /// Serialize [`Self::dispatch_overrides`] at the Rust/Python worker
    /// boundary.
    ///
    /// Returns an empty string when no overrides are set.
    pub fn to_dispatch_json_string(&self) -> String {
        let map = self.dispatch_overrides();
        if map.is_empty() {
            return String::new();
        }
        serde_json::to_string(&map).unwrap_or_else(|e| format!("<serialization failed: {e}>"))
    }
}

impl Serialize for EngineOverrides {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let count = self.asr.is_some() as usize
            + self.fa.is_some() as usize
            + self.utr.is_some() as usize
            + self.translate.is_some() as usize
            + self.extras.len();
        let mut map = serializer.serialize_map(Some(count))?;
        if let Some(ref asr) = self.asr {
            map.serialize_entry("asr", asr.as_wire_name())?;
        }
        if let Some(ref fa) = self.fa {
            map.serialize_entry("fa", fa.as_wire_name())?;
        }
        if let Some(ref utr) = self.utr {
            map.serialize_entry("utr", utr.as_wire_name())?;
        }
        if let Some(ref translate) = self.translate {
            map.serialize_entry("translate", translate.as_wire_name())?;
        }
        for (key, value) in &self.extras {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for EngineOverrides {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::deserialize(deserializer)?;
        let mut overrides = Self::default();
        for (key, value) in map {
            match key.as_str() {
                "asr" => {
                    overrides.asr = Some(
                        AsrEngineName::from_wire_name(&value).map_err(serde::de::Error::custom)?,
                    );
                }
                "fa" => {
                    overrides.fa = Some(
                        FaEngineName::from_wire_name(&value).map_err(serde::de::Error::custom)?,
                    );
                }
                "utr" => {
                    overrides.utr =
                        Some(UtrEngine::from_wire_name(&value).map_err(serde::de::Error::custom)?);
                }
                "translate" => {
                    overrides.translate = Some(
                        TranslateEngineName::from_wire_name(&value)
                            .map_err(serde::de::Error::custom)?,
                    );
                }
                _other => {
                    // Per-engine configuration knob. The set of valid
                    // keys is engine-specific and validated on the
                    // Python side at load time; an unknown knob falls
                    // through to engine defaults rather than rejecting
                    // the entire CLI invocation. See the doc comment
                    // on EngineOverrides for the rationale.
                    overrides.extras.insert(key, value);
                }
            }
        }
        Ok(overrides)
    }
}

#[cfg(test)]
mod tests {
    //! Wire-name / dispatch-key roundtrip coverage for ``AsrEngineName``.
    //!
    //! The wire name is the single source of truth shared between
    //! Rust (``AsrEngineName`` here, ``AsrBackendV2`` in
    //! ``batchalign-types``), Python (``AsrEngine`` enum in
    //! ``batchalign/worker/_types.py``), the CLI flag parser, and SQLite
    //! job persistence. A mismatch in any one of those locations breaks
    //! dispatch silently. These tests pin the contract at the Rust
    //! entry point.
    use super::*;

    #[test]
    fn whisper_hub_wire_roundtrip() {
        assert_eq!(AsrEngineName::WhisperHub.wire_name(), "whisper_hub");
        assert_eq!(
            AsrEngineName::try_from_wire_name("whisper_hub"),
            Some(AsrEngineName::WhisperHub),
        );
    }

    #[test]
    fn whisper_rs_wire_roundtrip() {
        assert_eq!(AsrEngineName::WhisperRs.wire_name(), "whisper_rs");
        assert_eq!(
            AsrEngineName::try_from_wire_name("whisper_rs"),
            Some(AsrEngineName::WhisperRs),
        );
    }

    #[test]
    fn whisper_rs_is_rust_owned_but_not_revai() {
        // The native whisper.cpp path runs in-process (Rust-owned), like
        // Rev.AI, so it has no pool-managed Python worker.
        assert!(AsrEngineName::WhisperRs.is_rust_owned());
        assert!(!AsrEngineName::WhisperRs.is_revai());
        assert_eq!(AsrEngineName::WhisperRs.dispatch_override_name(), None);
    }

    #[test]
    fn whisper_hub_is_not_rust_owned() {
        // Rust-owned engines run inference from the server process directly
        // (Rev.AI and the native whisper-rs path today). whisper_hub runs in
        // a Python worker like stock Whisper / WhisperX / HK engines.
        assert!(!AsrEngineName::WhisperHub.is_rust_owned());
        assert!(!AsrEngineName::WhisperHub.is_revai());
    }

    #[test]
    fn whisper_hub_dispatch_override_name_matches_wire_name() {
        // Worker pool keys must match the wire name so the Python worker
        // bootstrap sees ``engine_overrides["asr"] == "whisper_hub"`` and
        // routes to the fine-tune loader in ``_model_loading/asr.py``.
        assert_eq!(
            AsrEngineName::WhisperHub.dispatch_override_name(),
            Some("whisper_hub"),
        );
    }

    // ---- TranslateEngineName ----
    //
    // Pinned because the Python worker's `resolve_translate_engine`
    // (`batchalign/worker/_model_loading/translation.py`) matches on the
    // exact strings "google" and "seamless". A typo here would
    // silently fall through to the default engine on the Python side.

    #[test]
    fn translate_engine_google_wire_roundtrip() {
        assert_eq!(TranslateEngineName::Google.wire_name(), "google");
        assert_eq!(
            TranslateEngineName::try_from_wire_name("google"),
            Some(TranslateEngineName::Google),
        );
    }

    #[test]
    fn translate_engine_seamless_wire_roundtrip() {
        assert_eq!(TranslateEngineName::Seamless.wire_name(), "seamless");
        assert_eq!(
            TranslateEngineName::try_from_wire_name("seamless"),
            Some(TranslateEngineName::Seamless),
        );
    }

    #[test]
    fn translate_engine_nllb_wire_roundtrip() {
        assert_eq!(TranslateEngineName::Nllb.wire_name(), "nllb");
        assert_eq!(
            TranslateEngineName::try_from_wire_name("nllb"),
            Some(TranslateEngineName::Nllb),
        );
    }

    #[test]
    fn translate_engine_tencent_wire_roundtrip() {
        assert_eq!(TranslateEngineName::Tencent.wire_name(), "tencent");
        assert_eq!(
            TranslateEngineName::try_from_wire_name("tencent"),
            Some(TranslateEngineName::Tencent),
        );
    }

    #[test]
    fn translate_engine_aliyun_wire_roundtrip() {
        // Aliyun Machine Translation is the cloud-API translate engine
        // for Cantonese (``yue``) and other Asian-language source codes
        // that Tencent TMT does not list. The wire name ``"aliyun"``
        // must match the Python worker's ``TranslationBackend.ALIYUN``
        // value in ``batchalign/inference/_domain_types.py`` exactly,
        // since the resolver in
        // ``batchalign/worker/_model_loading/translation.py`` matches
        // on string equality.
        assert_eq!(TranslateEngineName::Aliyun.wire_name(), "aliyun");
        assert_eq!(
            TranslateEngineName::try_from_wire_name("aliyun"),
            Some(TranslateEngineName::Aliyun),
        );
    }

    #[test]
    fn translate_engine_unknown_wire_name_is_rejected() {
        assert_eq!(TranslateEngineName::try_from_wire_name("gogle"), None);
        let err = TranslateEngineName::from_wire_name("gogle").unwrap_err();
        assert_eq!(err.category, "translate");
        assert_eq!(err.name, "gogle");
    }

    #[test]
    fn translate_engine_resident_memory_ordering() {
        // Pins the physical ordering, Google (HTTP client) <
        // Seamless (~2.4 GB) < NLLB (~5 GB), that the admission-gate
        // engine-aware reservation
        // (``worker::pool::memory_gate::engine_aware_startup_reservation_mb``)
        // relies on. A typo here would silently re-introduce under-
        // reservation for the heavier engines.
        let google_mb = TranslateEngineName::Google.resident_memory_mb();
        let seamless_mb = TranslateEngineName::Seamless.resident_memory_mb();
        let nllb_mb = TranslateEngineName::Nllb.resident_memory_mb();
        assert!(
            google_mb < seamless_mb,
            "Google ({google_mb} MB) must be smaller than Seamless ({seamless_mb} MB)"
        );
        assert!(
            seamless_mb < nllb_mb,
            "Seamless ({seamless_mb} MB) must be smaller than NLLB ({nllb_mb} MB)"
        );
    }

    #[test]
    fn asr_engine_resident_memory_partitions_local_vs_cloud() {
        // Local Whisper variants must all match the heavy-model
        // footprint; cloud HTTP clients must all match the cheap
        // baseline. The admission gate's engine-aware reservation
        // depends on this partition being clean.
        assert_eq!(
            AsrEngineName::Whisper.resident_memory_mb(),
            WHISPER_LARGE_V3_RSS_MB
        );
        assert_eq!(
            AsrEngineName::WhisperHub.resident_memory_mb(),
            WHISPER_LARGE_V3_RSS_MB
        );
        assert_eq!(
            AsrEngineName::WhisperX.resident_memory_mb(),
            WHISPER_LARGE_V3_RSS_MB
        );
        for cloud in [
            AsrEngineName::RevAi,
            AsrEngineName::WhisperOai,
            AsrEngineName::HkTencent,
            AsrEngineName::HkAliyun,
            AsrEngineName::HkFunaudio,
        ] {
            assert_eq!(
                cloud.resident_memory_mb(),
                HTTP_CLIENT_BASELINE_RSS_MB,
                "{cloud:?} should match the cloud HTTP-client baseline"
            );
        }
        const _: () = assert!(HTTP_CLIENT_BASELINE_RSS_MB < WHISPER_LARGE_V3_RSS_MB);
    }

    #[test]
    fn asr_engine_qwen_wire_roundtrip() {
        // ``HkQwen`` wires as ``"qwen"`` across the JSON and the
        // engine-overrides knob. Round-trip pinned so a future rename
        // breaks visibly.
        let engine = AsrEngineName::HkQwen;
        assert_eq!(engine.wire_name(), "qwen");
        assert_eq!(
            AsrEngineName::try_from_wire_name("qwen"),
            Some(AsrEngineName::HkQwen)
        );
        assert_eq!(engine.dispatch_override_name(), Some("qwen"));
    }

    #[test]
    fn asr_engine_qwen_resident_memory_matches_local_model_footprint() {
        // Qwen3-ASR-1.7B is a local model, not a cloud HTTP client.
        // Its resident footprint must reserve enough headroom for the
        // weights + tokenizer + Python runtime. We pin it to the same
        // class as Whisper-large-v3, both are local ~1.5-3 GB
        // models with similar Python-side overhead. Wrong-side
        // partitioning (treating Qwen as a cloud HTTP client) would
        // under-reserve memory and trigger admission-gate OOM kills
        // on tight hosts.
        let qwen_mb = AsrEngineName::HkQwen.resident_memory_mb();
        assert!(
            qwen_mb >= WHISPER_LARGE_V3_RSS_MB,
            "Qwen ({qwen_mb} MB) must reserve at least the local-model baseline ({WHISPER_LARGE_V3_RSS_MB} MB)"
        );
        assert!(
            qwen_mb > HTTP_CLIENT_BASELINE_RSS_MB,
            "Qwen must NOT be partitioned as a cloud HTTP client ({HTTP_CLIENT_BASELINE_RSS_MB} MB)"
        );
    }

    #[test]
    fn fa_engine_resident_memory_separates_whisper_from_wave2vec() {
        assert_eq!(
            FaEngineName::Whisper.resident_memory_mb(),
            WHISPER_LARGE_V3_RSS_MB
        );
        assert_eq!(
            FaEngineName::Wave2Vec.resident_memory_mb(),
            WAVE2VEC_FA_RSS_MB
        );
        assert_eq!(
            FaEngineName::Wav2vecCanto.resident_memory_mb(),
            WAVE2VEC_FA_RSS_MB
        );
        const _: () = assert!(WAVE2VEC_FA_RSS_MB < WHISPER_LARGE_V3_RSS_MB);
    }

    #[test]
    fn translate_engine_tencent_matches_http_client_baseline() {
        // Tencent TMT is a thin HTTP-client engine, no local model
        // loaded, so its resident footprint is the same as Google's
        // and Seamless's lightweight baseline. Pinned to prevent
        // accidental inflation (which would over-reserve memory and
        // refuse spawns on hosts that can comfortably run Tencent
        // translate workers).
        assert_eq!(
            TranslateEngineName::Tencent.resident_memory_mb(),
            HTTP_CLIENT_BASELINE_RSS_MB
        );
        assert_eq!(
            TranslateEngineName::Tencent.resident_memory_mb(),
            TranslateEngineName::Google.resident_memory_mb()
        );
    }

    #[test]
    fn translate_engine_no_variant_is_rust_owned() {
        // All backends run in the Python worker, none talk to a
        // provider directly from the Rust server.
        assert!(!TranslateEngineName::Google.is_rust_owned());
        assert!(!TranslateEngineName::Seamless.is_rust_owned());
        assert!(!TranslateEngineName::Nllb.is_rust_owned());
        assert!(!TranslateEngineName::Tencent.is_rust_owned());
    }

    #[test]
    fn translate_engine_serializes_as_wire_string() {
        let json = serde_json::to_string(&TranslateEngineName::Seamless).unwrap();
        assert_eq!(json, "\"seamless\"");
    }

    #[test]
    fn translate_engine_deserializes_from_wire_string() {
        let parsed: TranslateEngineName = serde_json::from_str("\"seamless\"").unwrap();
        assert_eq!(parsed, TranslateEngineName::Seamless);
    }

    #[test]
    fn translate_engine_deserialize_rejects_unknown_variant() {
        let err = serde_json::from_str::<TranslateEngineName>("\"gogle\"").unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("gogle"),
            "expected error to mention the bad name, got: {message}"
        );
    }

    // ---- EngineOverrides translate field ----

    #[test]
    fn engine_overrides_serializes_translate_field() {
        let overrides = EngineOverrides {
            asr: None,
            fa: None,
            translate: Some(TranslateEngineName::Seamless),
            ..Default::default()
        };
        let json = overrides.to_json_string();
        assert_eq!(json, "{\"translate\":\"seamless\"}");
    }

    #[test]
    fn engine_overrides_deserializes_translate_field() {
        let parsed: EngineOverrides = serde_json::from_str("{\"translate\":\"seamless\"}").unwrap();
        assert_eq!(parsed.translate, Some(TranslateEngineName::Seamless));
        assert!(parsed.asr.is_none());
        assert!(parsed.fa.is_none());
    }

    #[test]
    fn engine_overrides_translate_only_is_not_empty() {
        let overrides = EngineOverrides {
            asr: None,
            fa: None,
            translate: Some(TranslateEngineName::Seamless),
            ..Default::default()
        };
        assert!(!overrides.is_empty());
    }

    #[test]
    fn engine_overrides_all_none_is_still_empty() {
        let overrides = EngineOverrides::default();
        assert!(overrides.is_empty());
        assert_eq!(overrides.to_json_string(), "");
    }

    // ---- EngineOverrides extras (per-engine knobs) ----

    #[test]
    fn engine_overrides_extras_round_trip_unknown_keys() {
        // Drill-down regression guard for Fix 1 (the starter test
        // lives in cli/args/tests.rs and exercises the full
        // Cli::parse_from → build_typed_options → to_json_string
        // path). This pins the deserialize/serialize layer in
        // isolation so a future refactor that moves the JSON shape
        // can't silently drop extras.
        let parsed: EngineOverrides = serde_json::from_str(
            r#"{"asr":"qwen","qwen_model":"Qwen/Qwen3-ASR-0.6B","qwen_device":"cuda"}"#,
        )
        .unwrap();
        assert_eq!(parsed.asr, Some(AsrEngineName::HkQwen));
        assert_eq!(
            parsed.extras.get("qwen_model").map(String::as_str),
            Some("Qwen/Qwen3-ASR-0.6B")
        );
        assert_eq!(
            parsed.extras.get("qwen_device").map(String::as_str),
            Some("cuda")
        );

        let json = parsed.to_json_string();
        let reparsed: EngineOverrides = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, reparsed, "round-trip must be lossless");
    }

    #[test]
    fn engine_overrides_extras_only_is_not_empty() {
        // An override payload of just per-engine knobs (no explicit
        // engine selection) is still a meaningful payload that must
        // reach the worker: ``is_empty`` must reflect that or the
        // ``--engine-overrides`` flag drops out before reaching the
        // worker spawn arg (see ``worker/handle/spawn.rs:61``).
        let parsed: EngineOverrides =
            serde_json::from_str(r#"{"qwen_model":"Qwen/Qwen3-ASR-0.6B"}"#).unwrap();
        assert_eq!(parsed.asr, None);
        assert!(!parsed.is_empty());
    }

    #[test]
    fn engine_overrides_known_engine_validation_still_fires() {
        // Unknown values for KNOWN keys (asr/fa/translate) still
        // error: Fix 1 relaxed schema strictness only for unknown
        // KEYS. A typo in an engine name is still loud.
        let err = serde_json::from_str::<EngineOverrides>(r#"{"asr":"wisper"}"#).unwrap_err();
        assert!(
            err.to_string().contains("wisper"),
            "expected engine-name validation error, got: {err}"
        );
    }
}

/// The name a user reaches for when they want Mandarin/Cantonese Paraformer.
///
/// Not an [`AsrEngineName`] variant: Paraformer is the FunAudio engine loading
/// a particular checkpoint, so promoting it to its own backend would duplicate
/// the funaudio dispatch path for no gain. It is a selection name instead.
pub const PARAFORMER_SELECTION_NAME: &str = "paraformer";

/// The FunASR checkpoint that makes FunAudio behave as Paraformer.
///
/// This is the checkpoint the FunASR ecosystem publishes under the Paraformer
/// name for Chinese, so `--asr-engine paraformer` and a hand-written
/// `funaudio_model=paraformer-zh` load exactly the same model.
pub const PARAFORMER_CHECKPOINT: &str = "paraformer-zh";

/// The override key FunAudio reads its checkpoint from.
pub const FUNAUDIO_MODEL_OVERRIDE_KEY: &str = "funaudio_model";

/// A selection that implies nothing beyond its engine.
const NO_IMPLIED_OVERRIDES: &[(&str, &str)] = &[];

/// What selecting `paraformer` implies: the FunASR checkpoint that makes the
/// FunAudio engine behave as Paraformer.
const PARAFORMER_IMPLIED_OVERRIDES: &[(&str, &str)] =
    &[(FUNAUDIO_MODEL_OVERRIDE_KEY, PARAFORMER_CHECKPOINT)];

/// An ASR engine choice as made by a user, with anything that choice implies.
///
/// Constructed only by [`parse`](Self::parse), so a selection cannot be
/// assembled from an engine and an unrelated set of overrides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsrSelection {
    engine: AsrEngineName,
    implied_overrides: &'static [(&'static str, &'static str)],
}

impl AsrSelection {
    /// A selection that is just an engine, with nothing implied.
    ///
    /// For engines known at COMPILE time, so they never travel through a
    /// string. The BA2 compatibility switches used this type by calling
    /// `parse("whisperx")` and friends, which meant a wire-name change would
    /// have turned four working flags into silent no-ops with nothing to catch
    /// it; taking the variant makes that a compile error instead.
    pub fn from_engine(engine: AsrEngineName) -> Self {
        Self {
            engine,
            implied_overrides: NO_IMPLIED_OVERRIDES,
        }
    }

    /// Resolve a user-facing engine name.
    ///
    /// Accepts every [`AsrEngineName`] wire name plus
    /// [`PARAFORMER_SELECTION_NAME`]. Returns `None` for anything else, and
    /// the caller must REPORT that rather than proceeding: an unrecognised
    /// engine name used to be swallowed by an `Option`-returning resolver, so a
    /// typo silently produced no engine at all.
    pub fn parse(name: &str) -> Option<Self> {
        if name == PARAFORMER_SELECTION_NAME {
            return Some(Self {
                engine: AsrEngineName::HkFunaudio,
                implied_overrides: PARAFORMER_IMPLIED_OVERRIDES,
            });
        }
        // The SAME table the CLI derives its hidden-alias list from. These were
        // two tables (a separate `LEGACY_SELECTION_ALIASES` const), so a
        // spelling could be advertised by one and rejected by the other, which
        // is precisely what `accepted_names` documents as impossible.
        AsrEngineName::resolve_variant(name).map(Self::from_engine)
    }

    /// The backend this selection runs on.
    pub fn engine(&self) -> AsrEngineName {
        self.engine.clone()
    }

    /// Overrides the NAME implies, which an explicit `--engine-overrides` wins
    /// over: the user's typed JSON is more specific than the name's default.
    pub fn implied_overrides(&self) -> &'static [(&'static str, &'static str)] {
        self.implied_overrides
    }

    /// Apply what the NAME implies, without beating what the user typed.
    ///
    /// Lives on the selection because the merge is part of what selecting a
    /// name MEANS. Every transcribing command had its own copy of this loop,
    /// which is the same shape the shared resolver removed one level up: an
    /// arm that forgets it makes `--asr-engine paraformer` parse and then run
    /// plain funaudio, which is the exact defect the name exists to fix.
    ///
    /// An explicit `--engine-overrides` wins: a user naming a checkpoint is
    /// being more specific than the name's default, not less.
    pub fn apply_implied(&self, overrides: &mut EngineOverrides) {
        for (key, value) in self.implied_overrides {
            overrides
                .extras
                .entry((*key).to_string())
                .or_insert_with(|| (*value).to_string());
        }
    }
}

#[cfg(test)]
mod asr_selection_tests {
    use super::*;

    /// Every engine in the owner list resolves by its own wire name.
    ///
    /// This is what keeps the CLI's accepted set honest: a new variant that is
    /// not in `ALL` is caught here rather than by a user who cannot reach it.
    #[test]
    fn every_engine_in_all_parses_from_its_wire_name() {
        for engine in AsrEngineName::ALL {
            let selection = AsrSelection::parse(engine.wire_name())
                .unwrap_or_else(|| panic!("{} must parse", engine.wire_name()));
            assert_eq!(&selection.engine(), engine);
            assert!(
                selection.implied_overrides().is_empty(),
                "a plain engine name implies no overrides"
            );
        }
    }

    /// Every entry in `ALL` names a distinct engine.
    ///
    /// The COUNT is carried by the array type (`[Self; 10]`), so adding a
    /// variant without extending `ALL` is a compile error and needs no test.
    /// What a type cannot check is that the ten entries are ten DIFFERENT
    /// engines, which is what this asserts.
    #[test]
    fn all_names_are_distinct() {
        let mut names: Vec<&str> = AsrEngineName::ALL.iter().map(|e| e.wire_name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), AsrEngineName::ALL.len(), "duplicate wire name");
        assert_eq!(names.len(), 10, "ALL must list all ten engines");
    }

    /// Paraformer resolves to funaudio carrying its checkpoint.
    #[test]
    fn paraformer_resolves_to_funaudio_plus_checkpoint() {
        let selection = AsrSelection::parse(PARAFORMER_SELECTION_NAME).expect("selectable");
        assert_eq!(selection.engine(), AsrEngineName::HkFunaudio);
        assert_eq!(selection.implied_overrides(), PARAFORMER_IMPLIED_OVERRIDES);
    }

    /// An unknown name yields nothing, so the caller has to report it.
    #[test]
    fn an_unknown_name_does_not_resolve() {
        assert_eq!(AsrSelection::parse("paraformr"), None);
        assert_eq!(AsrSelection::parse(""), None);
    }

    /// The user-facing list is the owner list plus paraformer, with no gaps.
    #[test]
    fn selectable_names_covers_all_engines_and_paraformer() {
        let names: Vec<&str> = AsrEngineName::selectable_names().collect();
        assert_eq!(names.len(), AsrEngineName::ALL.len() + 1);
        assert!(names.contains(&PARAFORMER_SELECTION_NAME));
        for engine in AsrEngineName::ALL {
            assert!(names.contains(&engine.wire_name()));
        }
    }
}

#[cfg(test)]
mod selectable_engine_tests {
    use super::*;

    /// Every engine in a category is advertised, resolves, and resolves back to
    /// itself, checked generically so no category can be left out.
    ///
    /// This is the property the four hand-written surfaces kept failing: three
    /// of four advertised a SUBSET of what they accepted. It survives as a test
    /// rather than a type because "the advertised list and the resolver agree"
    /// is a relationship between two functions, which no signature states.
    fn assert_category_is_coherent<E>()
    where
        E: SelectableEngine + PartialEq + std::fmt::Debug,
    {
        // Written against `resolve_variant`, which returns `Option<Self>` for
        // EVERY category including ASR. The first version was bounded on
        // `Selected = E`, which silently exempted ASR, the only category with
        // ten engines and the one whose list had already gone stale once.

        // The load-bearing check, and the ONLY one here that can catch an
        // engine missing from `ALL`. `ACCEPTED_NAMES` is written independently
        // of `ALL`, so it is a second witness; anything derived from `ALL` and
        // compared against `ALL` is circular. An earlier version asserted
        // `advertised.len() == ALL.len()`, which passed cleanly with a variant
        // deleted from `ALL`, because deleting it shrank both sides.
        for (name, engine) in E::accepted_names() {
            assert!(
                E::ALL.contains(engine),
                "{}: {name} resolves to {engine:?}, which is missing from ALL, \
                 so that engine is unreachable from the flag",
                E::CATEGORY
            );
        }

        // Every engine's canonical name is in the table. This is what makes
        // `accepted_names` mean the same thing in all four categories; three
        // of them used to omit some or all canonical names, which forced three
        // different resolvers and weakened the check above.
        for engine in E::ALL {
            let name = engine.selection_name();
            assert_eq!(
                E::resolve_variant(name).as_ref(),
                Some(engine),
                "{}: canonical name {name} must resolve back to itself",
                E::CATEGORY
            );
        }

        // Hidden aliases must resolve: clap rejects anything outside the
        // shown-plus-hidden list before the resolver runs, so an alias the
        // resolver does not know is a value clap accepts and then errors on.
        for alias in E::hidden_alias_names() {
            assert!(
                E::resolve_variant(alias).is_some(),
                "{}: hidden alias {alias} does not resolve",
                E::CATEGORY
            );
        }

        // The wire name round-trips, so persisted job options still parse.
        for engine in E::ALL {
            assert_eq!(
                E::try_from_wire_name(engine.wire_name()).as_ref(),
                Some(engine),
                "{}: wire name {} must parse back",
                E::CATEGORY,
                engine.wire_name()
            );
        }

        // One canonical name per engine.
        let mut advertised: Vec<&str> = E::ALL.iter().map(E::selection_name).collect();
        let count = advertised.len();
        advertised.sort_unstable();
        advertised.dedup();
        assert_eq!(
            advertised.len(),
            count,
            "{}: two engines share a selection name",
            E::CATEGORY
        );
    }

    #[test]
    fn utr_category_is_coherent() {
        assert_category_is_coherent::<UtrEngine>();
    }

    #[test]
    fn fa_category_is_coherent() {
        assert_category_is_coherent::<FaEngineName>();
    }

    #[test]
    fn translate_category_is_coherent() {
        assert_category_is_coherent::<TranslateEngineName>();
    }

    /// ASR is no longer exempt.
    #[test]
    fn asr_category_is_coherent() {
        assert_category_is_coherent::<AsrEngineName>();
    }

    /// ASR additionally advertises `paraformer`, a selection rather than a
    /// variant, and it must resolve through the CLI's own resolver.
    #[test]
    fn asr_advertises_paraformer_on_top_of_its_engines() {
        let advertised: Vec<&str> = AsrEngineName::selectable_names().collect();
        assert_eq!(advertised.len(), AsrEngineName::ALL.len() + 1);
        assert!(advertised.contains(&PARAFORMER_SELECTION_NAME));
        for name in &advertised {
            assert!(
                AsrEngineName::resolve(name).is_some(),
                "advertised {name} does not resolve"
            );
        }
    }

    /// The default aligner must report word intervals, not bare onsets.
    ///
    /// POLICY: an onset-only engine is a legitimate engine, it just cannot be
    /// the default without an onset-to-interval step, because a word's end
    /// would otherwise equal its start. Asserting the PROPERTY rather than a
    /// particular variant means a new interval-reporting engine may become the
    /// default without editing this test, while an onset-only one may not.
    #[test]
    fn default_fa_engine_reports_word_intervals() {
        assert_eq!(
            FaEngineName::DEFAULT.timing_resolution(),
            FaTimingResolution::WordIntervals,
            "the default aligner must report a word's end, not only its start"
        );
    }

    /// Every FA engine reports the shape its model actually produces.
    ///
    /// `Wav2vecCanto` is the case worth pinning: it is a wav2vec model and
    /// returns index-aligned word spans, but its wire name is `cantonese_fa`.
    /// Classification used to be a substring test for "wav2vec" on that name,
    /// so it fell through to the onset-only branch and its word spans were
    /// read as token onsets on the wrong grouping window.
    #[test]
    fn every_fa_engine_reports_the_shape_its_model_produces() {
        for (engine, expected) in [
            (FaEngineName::Wave2Vec, FaTimingResolution::WordIntervals),
            (
                FaEngineName::Wav2vecCanto,
                FaTimingResolution::WordIntervals,
            ),
            (FaEngineName::Whisper, FaTimingResolution::TokenOnsets),
        ] {
            assert_eq!(
                engine.timing_resolution(),
                expected,
                "{} classified wrongly",
                engine.wire_name()
            );
        }
    }

    /// The default is a member of its own category.
    #[test]
    fn every_default_is_an_engine_that_exists() {
        assert!(UtrEngine::ALL.contains(&UtrEngine::DEFAULT));
        assert!(FaEngineName::ALL.contains(&FaEngineName::DEFAULT));
        assert!(AsrEngineName::ALL.contains(&AsrEngineName::DEFAULT));
        assert!(TranslateEngineName::ALL.contains(&TranslateEngineName::DEFAULT));
    }
}
