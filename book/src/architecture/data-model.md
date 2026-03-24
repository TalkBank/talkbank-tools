# Data Model

**Status:** Current
**Last updated:** 2026-03-22

The `talkbank-model` crate defines the typed AST for CHAT files. It is the central crate that all other crates depend on.

## ChatFile

The root type is `ChatFile`, representing a complete CHAT transcript:

```rust
pub struct ChatFile {
    pub headers: Headers,
    pub utterances: Vec<Utterance>,
}
```

The following diagram shows the full ownership tree — what each type
contains and how the pieces compose into a complete CHAT file.

```mermaid
flowchart TD
    chatfile["ChatFile\n(chat_file/core.rs)"]
    lines["lines: Vec&lt;Line&gt;"]
    participants["participants:\nIndexMap&lt;SpeakerCode, Participant&gt;"]
    languages["languages: LanguageCodes"]
    options["options: ChatOptionFlags"]

    chatfile --> lines & participants & languages & options

    header_line["Line::Header\n(Header)"]
    utt_line["Line::Utterance\n(Utterance)"]
    lines --> header_line & utt_line

    main["main: MainTier\n(main_tier.rs)"]
    deptiers["dependent_tiers:\nVec&lt;DependentTier&gt;"]
    health["parse_health:\nParseHealthState"]
    preceding["preceding_headers:\nSmallVec&lt;Header&gt;"]

    utt_line --> preceding & main & deptiers & health

    speaker["speaker: SpeakerCode"]
    tiercontent["content: TierContent"]
    main --> speaker & tiercontent

    linkers["linkers: Vec&lt;Linker&gt;"]
    uttcontent["utterance_content:\nVec&lt;UtteranceContent&gt;\n(24 variants)"]
    terminator["terminator:\nOption&lt;Terminator&gt;"]
    bullet["bullet: Option&lt;Bullet&gt;"]
    tiercontent --> linkers & uttcontent & terminator & bullet

    subgraph "DependentTier — 25 variants (dependent_tier/types.rs)"
        structured["Structured linguistic:\nMor Gra Pho Mod\nSin Act Cod Wor"]
        bulletcontent["With inline bullets:\nAdd Com Exp Gpx\nInt Sit Spa"]
        textonly["Text-only:\nAlt Coh Def Eng Err\nFac Flo Gls Ort Par Tim"]
        phon["Phon project:\nModsyl Phosyl Phoaln"]
        userdefined["UserDefined\nUnsupported"]
    end

    deptiers --> structured & bulletcontent & textonly & phon & userdefined
```

## Key Types

### Headers

```rust
pub struct Headers {
    pub participants: Vec<Participant>,
    pub languages: Vec<Language>,
    pub ids: Vec<IdHeader>,
    pub date: Option<Date>,
    pub media: Option<Media>,
    pub comments: Vec<Comment>,
    // ... other optional headers
}
```

### Utterance

```rust
pub struct Utterance {
    pub speaker: Speaker,
    pub content: MainTierContent,
    pub terminator: Terminator,
    pub dependent_tiers: Vec<DependentTier>,
    pub bullet: Option<Bullet>,
}
```

### Dependent Tiers

The `DependentTier` enum covers all tier types:

```rust
pub enum DependentTier {
    Mor(MorTier),
    Gra(GraTier),
    Pho(PhoTier),
    Wor(WorTier),
    Sin(SinTier),
    Com(ComTier),
    Act(ActTier),
    // ... 20+ variants total
}
```

### MOR Tier

The MOR tier has a rich type hierarchy:

```rust
pub struct MorTier {
    pub tier_type: MorTierType,
    pub items: Vec<Mor>,
    pub terminator: Option<String>,
}

pub struct Mor {
    pub main: MorWord,
    pub post_clitics: Vec<MorWord>,
}

pub struct MorWord {
    pub pos: PosCategory,
    pub lemma: MorStem,
    pub features: SmallVec<[MorFeature; 4]>,
}
```

`MorFeature` supports both flat values (`Plur`) and Key=Value pairs (`Number=Plur`). See [The %mor Tier](../chat-format/mor-tier.md) for details.

## Interning

String-heavy types like `PosCategory`, `MorStem`, and `MorFeature` use `Arc<str>` with a global interner for deduplication. This significantly reduces memory usage when processing large corpora where the same POS tags and lemmas appear thousands of times.

## Validation

The model includes a validation system that checks semantic constraints beyond what the grammar enforces:

- `%mor` alignment: number of MOR items must match alignable main tier words
- `%gra` structure: sequential indices, ROOT checks, circular dependency detection
- Header consistency: `@ID` codes match `@Participants`
- Speaker references: all `*SPEAKER:` codes declared

Validation runs after parsing via `validate_with_alignment()`.

### Tier Alignment Flows

Each dependent tier aligns to the main tier (or to %mor) using domain-specific logic:

```mermaid
flowchart TD
    main["MainTier content"]
    walker["walk_words()\ncount alignable words"]

    subgraph "5 Parallel Alignment Flows"
        mor["%mor\nCustom logic\n(clitic handling)"]
        pho["%pho\npositional_align()\n(skip PhoGroup)"]
        sin["%sin\npositional_align()\n(skip SinGroup)"]
        wor["%wor\npositional_align()\n(LCS diff format)"]
        gra["%gra\nalign to %mor chunks\n(not main tier)"]
    end

    main --> walker
    walker --> mor & pho & sin & wor
    mor --> gra
```

## Serialization

### CHAT (WriteChat)

The `WriteChat` trait serializes model types back to CHAT format:

```rust
pub trait WriteChat {
    fn write_chat(&self, writer: &mut impl Write) -> io::Result<()>;
}
```

### JSON (Serde)

All model types implement `Serialize`/`Deserialize` for JSON roundtrip. The JSON format follows the [JSON Schema](../integrating/json-schema.md).

### Schema (Schemars)

Model types derive `JsonSchema` for automatic JSON Schema generation. Run `cargo test --test generate_schema` to regenerate `schema/chat-file.schema.json`.

## Content Walker

```mermaid
flowchart TD
    input["&[UtteranceContent]\n+ domain: Option<TierDomain>"]
    dispatch["Match variant\n(24 UtteranceContent variants)"]
    word["Word → emit WordItem::Word"]
    rw["ReplacedWord → emit WordItem::ReplacedWord"]
    sep["Separator → emit WordItem::Separator"]
    group["Group / AnnotatedGroup /\nPhoGroup / SinGroup / Quotation"]
    gate{"Domain\ngating"}
    skip["Skip\n(atomic unit)"]
    recurse["Recurse into\ngroup.content"]

    input --> dispatch
    dispatch --> word & rw & sep & group
    group --> gate
    gate -->|"Mor: skip retraces"| skip
    gate -->|"Pho/Sin: skip groups"| skip
    gate -->|"None: recurse all"| recurse
    recurse -->|back| dispatch
```

The walker is a two-layer design: `walk_content` (generic, all items) and `walk_words` (filtered to words/separators). Groups are descended transparently, and `AnnotatedWord`/`Event`/`Action` are unwrapped automatically. The `talkbank-model` crate provides `walk_words` as the primary closure-based walker, centralizing the recursive traversal of `UtteranceContent` (24 variants) and `BracketedItem` (22 variants).

```rust
pub enum WordItem<'a> {
    Word(&'a Word),
    ReplacedWord(&'a ReplacedWord),
    Separator(&'a Separator),
}

pub fn walk_words(
    content: &[UtteranceContent],
    domain: Option<TierDomain>,
    f: &mut impl FnMut(WordItem<'_>),
);

pub fn walk_words_mut(
    content: &mut [UtteranceContent],
    domain: Option<TierDomain>,
    f: &mut impl FnMut(WordItemMut<'_>),
);
```

**Domain-aware gating** is built into the walker:
- `domain = Some(Mor)` → skip `Retrace` content (retraced words are not morphologically analyzed)
- `domain = Some(Pho|Sin)` → skip `PhoGroup`/`SinGroup` (treated as atomic units)
- `domain = None` → recurse all groups unconditionally

The walker handles all 6 group types (`Group`, `AnnotatedGroup`, `Retrace`, `PhoGroup`, `SinGroup`, `Quotation`) and their `BracketedContent` recursion. Callers provide only leaf-handling logic: `counts_for_tier()` filtering, `ReplacedWord` branch logic, separator filtering.

Used by `main_tier.rs` (%wor generation) and `batchalign-chat-ops` (word extraction, FA extraction/injection/postprocess).

**Not suitable for**: `strip_timing_from_content()` (container mutation via `retain()`) or `count.rs` (Pho/Sin domains treat PhoGroup/SinGroup as counted atomic units).

### Overlap Marker Iterator

A second closure-based iterator visits CA overlap markers (⌈⌉⌊⌋) at all
three content levels — `UtteranceContent`, `BracketedItem`, and
`WordContent` (for intra-word markers like `butt⌈er⌉`):

```rust
pub fn walk_overlap_points(
    content: &[UtteranceContent],
    visitor: &mut impl FnMut(OverlapPointVisit<'_>),
);
```

For region-level analysis (pairing ⌈ with ⌉ by index), use
`extract_overlap_info()` which returns `OverlapMarkerInfo` with
properly paired `OverlapRegion` structs. See [Alignment](alignment.md#overlap-marker-iteration) for details.

## SmallVec Optimization

Collections that are typically small use `SmallVec` for inline storage:
- `SmallVec<[MorFeature; 4]>` — features per word (usually 0-4)
- `SmallVec<[MorWord; 2]>` — post-clitics (usually 0-1)

This avoids heap allocation for the common case.
