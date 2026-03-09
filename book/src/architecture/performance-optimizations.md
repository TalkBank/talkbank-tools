# Performance Optimizations

This document records the design rationale behind performance optimizations
applied to talkbank-tools. Each section explains the
problem, the alternatives considered, and why the chosen approach was taken.

Each optimization is documented inline below.

---

## Word::cleaned_text() — OnceLock\<SmolStr\> Cache

**Problem.** `cleaned_text()` allocates a new `String` on every call by
iterating word content and concatenating `Text`/`Shortening` variants.
Called 2–4× per word during validation and alignment. Across 95K files with
millions of words, this is the single largest allocation hotspot.

**Alternatives considered:**

| Approach | Pros | Cons |
|----------|------|------|
| Compute once at top of `validate()`, pass `&str` down | Zero struct overhead | Requires threading an extra parameter through every validator; breaks the `Validate` trait signature |
| `Option<String>` field, populate manually | Simple | Mutable, not thread-safe, callers must remember to populate |
| `OnceLock<String>` | Thread-safe, lazy | `String` still heap-allocates for short words |
| **`OnceLock<SmolStr>`** | Thread-safe, lazy, inline storage ≤23 bytes | Extra dependency (`smol_str`); 24 bytes per word even if never accessed |

**Decision.** `OnceLock<SmolStr>`. Most CHAT words are short enough to fit in
SmolStr's inline buffer (≤23 bytes), eliminating heap allocation entirely for
the common case. The 24-byte per-word overhead is negligible compared to the
rest of `Word` (300+ bytes).

**Implementation details:**

- A `CachedStr` newtype wraps `OnceLock<SmolStr>` with a custom `PartialEq`
  (always returns `true`) so the cache field does not affect equality checks
  or test assertions. Without this, every `Word` comparison would need the
  cache pre-populated to the same state.
- `serde(skip)`, `schemars(skip)`, `semantic_eq(skip)`, `span_shift(skip)`
  attributes exclude the field from serialization, JSON schema, semantic
  equality, and span-shift derive macros.
- Return type changed from `String` to `&str` — a breaking change for callers,
  but nearly all already used the result as `&str`. The few that stored it
  added `.to_string()`.

**Files:** `crates/talkbank-model/src/model/content/word/types.rs`

---

## ValidationContext — Arc\<SharedValidationData\> Split

**Problem.** `ValidationContext` is a 200+ byte struct containing
`HashSet<SpeakerCode>`, `Vec<LanguageCode>`, `ValidationConfig`, and several
boolean flags. It is cloned 3+ times per utterance (once per main tier, once
per word tier, once per dependent tier) and again for every word. The
file-level fields (participants, languages, options) are constants set once
from headers and never change during validation — they are deep-copied
unnecessarily.

**Alternatives considered:**

| Approach | Pros | Cons |
|----------|------|------|
| Pass file-level data as separate `&SharedData` parameter | Zero clone cost | Every `validate()` signature gains a parameter; breaks `Validate` trait |
| `Rc<SharedData>` | Cheap clone | Not `Send`; validation may run on rayon threads |
| **`Arc<SharedData>`** | Cheap clone, `Send + Sync` | One pointer indirection for field access |
| Make ValidationContext `Copy` by replacing collections with indices | Fastest possible clone | Requires a separate arena/registry, heavy refactor |

**Decision.** `Arc<SharedValidationData>`. Seven file-level-constant fields
(`participant_ids`, `default_language`, `declared_languages`, `ca_mode`,
`enable_quotation_validation`, `bullets_mode`, `config`) moved into an
`Arc`-wrapped struct. Five per-tier mutable "overlay" fields (`tier_language`,
`field_span`, `field_text`, `field_label`, `field_error_code`) stay inline.

Cloning a `ValidationContext` now copies the `Arc` pointer (8 bytes) + 5 small
overlay fields, instead of deep-cloning a `HashSet`, two `Vec`s, and a
`ValidationConfig`.

**Builder methods** use `Arc::make_mut()` for shared fields (copy-on-write if
the `Arc` is shared, no-op if unique). This keeps the builder API unchanged
while enabling cheap cloning once construction is complete.

**Files:** `crates/talkbank-model/src/validation/context.rs`, plus all
validators that access `context.participant_ids` etc. (now `context.shared.participant_ids`).

---

## Arc\<ChatFile\> in LSP DashMap

**Problem.** Every LSP request handler (hover, completion, symbols, folding,
inlay hints, formatting — ~10 handlers) calls `get_chat_file()` which clones
the entire `ChatFile` AST from the `DashMap`. For large files with hundreds of
utterances, this is significant waste since all handlers only read.

**Alternatives considered:**

| Approach | Pros | Cons |
|----------|------|------|
| Return `Ref<'_, Url, ChatFile>` (DashMap guard) | Zero-copy read | Holds shard lock for duration of request; blocks concurrent writes and other readers on the same shard |
| **`DashMap<Url, Arc<ChatFile>>`** | Pointer-copy read, no lock held | Mutation path must replace the entire `Arc` atomically |
| `im::HashMap` (persistent data structure) | Structural sharing | Heavy dependency, unfamiliar API, no DashMap integration |

**Decision.** `DashMap<Url, Arc<ChatFile>>`. Request handlers get
`Arc::clone()` (pointer copy), releasing the DashMap shard lock immediately.

The mutation path (validation orchestrator) needs an owned `ChatFile` for
alignment computation. It extracts the old file via
`ChatFile::clone(entry.value())`, which is a deep clone — but this only
happens on file change (debounced), not on every request. After mutation, the
new `ChatFile` is wrapped in `Arc::new()` and inserted atomically.

**Files:** `crates/talkbank-lsp/src/backend/state.rs`,
`requests.rs`, `validation_orchestrator.rs`

---

## Quick Wins (Previously Implemented)

These were implemented in the first performance pass and are documented here
for completeness.

**Thread-local parser pool** (`with_parser()`): `TreeSitterParser` is expensive
to create (~1ms for tree-sitter init). A `thread_local!` pool reuses parsers
across calls within the same thread, eliminating per-call construction in
`parse_and_validate()`.

**SQLite PRAGMAs**: Added `synchronous=NORMAL`, `cache_size=-8000` (8MB),
`busy_timeout=5000`, `mmap_size=268435456` (256MB) to the validation cache.
Combined 2–3× write throughput improvement with no durability risk (WAL mode
is already crash-safe).

**Release profile**: Added `lto = "thin"`, `codegen-units = 1`,
`strip = "symbols"` to the workspace profile. ~15% throughput improvement for
CLI validation.

**LSP LineIndex**: Replaced O(n) `offset_to_position` with a pre-computed line
offset index using O(log n) binary search. Eliminates O(n²) behavior for
semantic tokens on large files.

**`did_save` skip**: LSP skips re-validation on save if the document hasn't
changed since the last debounced `did_change` validation.

**Per-thread SQLite connections** (`CachePool`): Replaced `Arc<Mutex<Connection>>`
with `thread_local::ThreadLocal<CacheConnection>`. Each worker thread lazily
opens its own SQLite connection. WAL mode handles concurrency natively —
concurrent readers, serialized writers with `busy_timeout` retry. No Mutex
anywhere. Cache hits are now contention-free.

**Pass/fail-only cache**: Dropped the `errors` table (10+ columns per error
including miette-rendered diagnostics) and the `corpora` table. The cache now
stores only a boolean per file: valid or invalid. This eliminated all
`ParseError` serialization/deserialization from cache operations and reduced
the DB from ~100MB to ~2MB for 95K files.

**Audit mode parallelization**: `run_audit_mode()` uses crossbeam worker
threads with a bounded work-stealing channel, matching the regular validation
path. Workers report completed file results to a dedicated audit writer thread,
which writes JSONL results to disk immediately and owns the summary counters.

**Roundtrip parse-only**: `run_roundtrip()` re-parses the serialized output
with `ParseValidateOptions::default()` (parse only, no validation). Roundtrip
checks serialization fidelity, not content validity — the caller already
ensures validation passed before invoking roundtrip.
