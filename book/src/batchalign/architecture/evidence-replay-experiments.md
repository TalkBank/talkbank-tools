# Evidence, Replay, and Experiment Topology

**Status:** Current
**Last updated:** 2026-08-30 21:00 EDT

This chapter is the visual map for BA3's v0.3 evidence architecture. Detailed
contracts remain in [Audio-Task Cache](../../architecture/runtime/audio-task-cache.md),
[Observability](observability.md), and the developer references for
[transcribe](../developer/commands/transcribe.md) and
[align](../developer/commands/align.md).

The central design rule is that acquiring model evidence, projecting that
evidence through local algorithms, and judging transcript quality are separate
operations. BA3 now constrains the first two. Human or corpus-specific
adjudication remains an experiment-layer responsibility.

## Implemented evidence lanes

```mermaid
flowchart TB
    MEDIA["Media bytes"]
    CHATIN["Existing CHAT"]

    subgraph PAID["Remote or model inference boundaries"]
        REV["Rev raw ASR evidence"]
        SPK["Raw speaker evidence"]
        FAW["Raw FA worker evidence"]
        UTW["Boundary-model evidence"]
    end

    subgraph LOCAL["Versioned local projections"]
        ASRP["ASR cleanup and timed chunks"]
        SPKP["Normalized turns and speaker projection"]
        FAP["Word timings and %wor policy"]
        UTP["Pre-CHAT and post-CHAT boundaries"]
    end

    subgraph OUTPUTS["Durable experiment products"]
        CACHE["Content-addressed raw/derived cache"]
        SIDE["Causal evidence sidecars"]
        REPLAY["Fingerprint-admitted replay bundle"]
        OUT["Validated CHAT"]
    end

    MEDIA --> REV --> ASRP
    MEDIA --> SPK --> SPKP
    MEDIA --> FAW --> FAP
    CHATIN --> FAP
    ASRP --> UTW --> UTP
    ASRP --> SPKP --> UTP --> OUT
    FAP --> OUT

    REV -.-> CACHE
    SPK -.-> CACHE
    FAW -.-> CACHE
    REV -.-> SIDE
    SPK -.-> SIDE
    FAW -.-> SIDE
    UTW -.-> SIDE
    ASRP -.-> REPLAY
    SPKP -.-> REPLAY
    OUT -.-> REPLAY
```

Solid arrows are semantic processing. Dashed arrows are retained evidence or
replay products. Sidecars are files, not CHAT dependent tiers: current BA3 does
not generate `%xalign` or `%xrev`.

## Inference authorization is a state transition

A cache miss is not permission to call a provider. The resolver must consume
the miss into a single-use authorization, and successful evidence must be
validated and committed durably before projection succeeds.

```mermaid
stateDiagram-v2
    [*] --> RequestIdentity: validate media and request facts
    RequestIdentity --> CompletedEvidence: admitted durable hit
    RequestIdentity --> CacheMiss: absent or deliberate refresh
    RequestIdentity --> Refused: corrupt or incompatible evidence
    CacheMiss --> Refused: RequireCache
    CacheMiss --> AuthorizedRun: UseCache or SkipCache
    AuthorizedRun --> VerifiedRun: media digest reverified
    VerifiedRun --> CompletedEvidence: provider/model result validated and committed
    VerifiedRun --> Refused: media drift, invalid result, or commit failure
    CompletedEvidence --> CurrentProjection
    CurrentProjection --> [*]
    Refused --> [*]
```

This is a typestate boundary. Provider adapters cannot manufacture
`AuthorizedRun`, and offline replay cannot accidentally acquire provider-call
capability.

## Replay has two deliberately different meanings

```mermaid
flowchart LR
    subgraph CACHE_REPLAY["Raw-evidence replay"]
        CR["Validated raw cache envelope"] --> CA["Re-admit against current request"]
        CA --> CP["Run current Rust projection"]
    end

    subgraph BUNDLE_REPLAY["Offline transcribe replay"]
        BM["Immutable manifest"] --> BF{"Verify media and artifact fingerprints"}
        BF -->|match| BA["Admitted projected ASR and turns"]
        BF -->|drift| BX["Refuse before output/model load"]
        BA --> BP["Run current downstream CHAT logic"]
    end
```

Raw-evidence replay can test a changed local normalizer or aligner projection.
The current offline transcribe bundle begins from retained projected ASR and
turn artifacts, so it tests downstream speaker projection, segmentation, CHAT
construction, and postprocessing without claiming that those artifacts are raw
Rev or raw pyannote evidence.

## Reproducible comparative experiment loop

```mermaid
flowchart TD
    Q["Narrow quality question"] --> C["Frozen troublesome clips<br/>and reference annotations"]
    C --> B["Capture baseline identities,<br/>requests, raw outputs, and CHAT"]
    B --> M["Fingerprint manifest"]
    M --> V1["Projection/policy variant A"]
    M --> V2["Projection/policy variant B"]
    V1 --> CMP["Machine comparison:<br/>words, speakers, boundaries, timings"]
    V2 --> CMP
    CMP --> H["Blind human adjudication<br/>with uncertainty/notes"]
    H --> D{"Evidence supports change?"}
    D -->|yes| T["Regression test + implementation + docs"]
    D -->|no| N["Record negative or inconclusive result"]
    T --> R["Code review and exact-commit gates"]
    R --> M2["New versioned projection identity"]
    M2 --> Q
    N --> Q
```

This loop is the basis for precise comparisons with the upstream BA3 fork and for
segmentation, diarization, Rev-media, and `%wor` studies. A system-level claim
requires the whole chain; a few plausible transcripts do not establish
universal superiority.

## Boundary with IISRP and MichiganChild merge work

The following is the intended downstream research topology, not a feature the
BA3 v0.3 CLI currently performs:

```mermaid
flowchart LR
    MAN["Imperfect child-only<br/>manual CHAT"]
    FULL["BA3 full-audio candidates<br/>words, speakers, boundaries, timings"]
    ACOU["Acoustic signals<br/>pitch, overlap, pauses"]
    SEM["Semantic signals<br/>fuzzy match, echo, lexical context"]
    REC["Typed reconciliation candidates"]
    SCORE["Auditable holistic scoring"]
    AUTO{"Confidence / ambiguity state"}
    MERGE["Merged transcript candidate"]
    REVIEW["Targeted human review"]
    FINAL["Validated delivery CHAT"]

    MAN --> REC
    FULL --> REC
    ACOU --> SCORE
    SEM --> SCORE
    REC --> SCORE --> AUTO
    AUTO -->|high confidence| MERGE
    AUTO -->|ambiguous| REVIEW --> MERGE
    MERGE --> FINAL
```

The manual child transcript is evidence, not an oracle: it may contain `xxx`,
miss adult interruptions, or choose different but defensible utterance
boundaries. The merge layer should therefore preserve competing candidates and
ambiguity until a typed decision is made. BA3 supplies replayable full-audio
evidence; project tooling performs the corpus-specific reconciliation.

## Diagram maintenance rule

When a new cache state, evidence artifact, inference capability, or projection
revision is added, update the smallest detailed diagram and this overview in
the same change. A diagram is part of the contract: if it cannot distinguish
raw evidence from a derived artifact or implemented behavior from planned
research, it is misleading and must not be marked current.
