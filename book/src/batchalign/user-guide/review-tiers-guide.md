# Review Tiers: `%xalign` and `%xrev`

**Status:** Current
**Last updated:** 2026-06-15 13:21 EDT

When batchalign3 makes a decision that could be wrong (clamping a timestamp,
filling a gap between utterances, stripping non-monotonic timing, or failing to
map a Stanza token back to a CHAT word), it can record the decision in a
`%xalign` tier and, on the subset that warrant human attention, flag a
paired `%xrev: [?]` marker. Together these tiers make an aligned CHAT file
**self-documenting for one review cycle**: a reviewer can see what the
algorithm did and why, then confirm, correct, or overrule the decision.

This page is the reviewer's guide to that workflow.

> **Off by default.** As of 2026-06-15, batchalign3 does **not** write
> `%xalign` / `%xrev` tiers unless you ask for them. They are unfinished
> review scaffolding, and leaving them in finished files just creates
> cleanup work, so emission is opt-in (no tiers are written at the default
> `none` level):
>
> - `align`: pass `--review-level low-confidence` (or `all`). The default
>   is `none`.
> - `morphotag`: pass `--review-level low-confidence` (or `all`), or set
>   the `review_level` field in the job options. The default is `none`, and
>   the decision tiers are produced only on morphotag's incremental
>   reprocessing path (`--before`).
>
> The decision-recording machinery is fully retained; only the emission is
> gated, so the feature can be turned back on per-run at any time. The rest
> of this page describes the review workflow that applies *once you have
> opted in*.

## Read this first: the tiers are short-lived scaffolding

`%xalign` and `%xrev` are **not long-lived CHAT annotations**. They are a
snapshot of a specific algorithm's reasoning at a specific moment. They are
only trustworthy while three things hold:

1. The align (or morphotag) run that emitted them is recent.
2. The main tiers have not been manually edited since that run.
3. The align algorithm has not changed since that run.

If any of these breaks, the tiers are **stale** — the `%xalign` reasoning
may reference decisions the current algorithm would no longer make, or
point to utterances whose content has shifted. A reviewer who works through
stale tiers is wasting their own time.

**How to spot stale tiers**:

- Look at the `@Comment: [ba3 align | ... | <timestamp>]` line near the
  top of the file. If the file's last modification time (`ls -l FILE.cha`)
  is **after** that timestamp, someone has edited the main tiers since
  align ran. The `%xalign` entries are stale.
- Look at the `@Comment` for the engine/algorithm version. If it differs
  from what the current batchalign3 version emits, the tiers predate the
  current algorithm.
- Going forward, every `%xalign` entry will carry its own inline algorithm
  version tag (e.g. `%xalign: monotonicity:end_clamped@algo-v3.2 ...`) so
  staleness can be read off each entry individually. Until that ships, the
  `@Comment`-vs-mtime check is the authoritative signal.

**What to do with stale tiers**: rerun `align` (or `morphotag`) on the
file. The rerun strips all existing `%x*` tiers and emits fresh ones
against the current algorithm and current file content. Never review a
stale tier.

## What the tiers look like

A decision tier entry is written directly below the utterance it applies to:

```chat
*CHI:   hello world . •1000_5000•
%xalign:	monotonicity:end_clamped overlap=1200ms prev_end=6200 next_start=5000
%xrev:	[?]
```

`%xalign` content follows the shape `module:strategy reason_string`, where
`reason_string` is a space-separated list of `key=value` pairs. `%xrev`
content is one short bracket-marker from the review vocabulary below, with an
optional free-text note after it.

## The review marker vocabulary

Reviewers replace the `[?]` in `%xrev` with one of:

| Marker | Meaning |
|---|---|
| `[ok]` | The algorithm's decision was correct. No change needed. |
| `[early]` | The bullet on the `*` tier starts earlier than it should. |
| `[late]` | The bullet on the `*` tier starts later than it should. |
| `[wrong]` | The decision was wrong but the reviewer has not yet fixed it. Leaves the file in a known-broken state. |
| `[corrected]` | The decision was wrong and the reviewer has fixed the `*` tier (e.g., moved or replaced a bullet). |
| `[stamped]` | The reviewer manually placed timing on an utterance that had none. |

A short free-text note may follow the marker, e.g. `[corrected] bullet was ~200ms late`.

## Decision modules

The `module` prefix on `%xalign` content tells the reviewer which pipeline
stage raised the decision. All decisions currently land in a single unified
`%xalign` tier regardless of module.

All four module prefixes only land in output when review level is above
`none` (off by default; see the callout at the top).

| Module prefix | Source stage | Status | Typical strategies |
|---|---|---|---|
| `fa:` | Forced alignment (`align`) | Opt-in via `--review-level` | `gap_filled`, `boundary_averaged`, `lis_removal`, `words_timing_dropped` |
| `monotonicity:` | Timing sanity (`align`) | Opt-in via `--review-level` | `end_clamped`, `timing_stripped` |
| `utr:` | Utterance timing recovery (`align` pre-pass) | Opt-in via `--review-level` | `unmatched`, `zero_duration_skipped` |
| `morphosyntax:` | Stanza mapping (`morphotag`) | Opt-in via the `review_level` option, **incremental path only**: emitted by `process_morphosyntax_incremental` (`crates/batchalign/src/morphosyntax/mod.rs`). The batched / full-file morphotag path does not inject decision tiers. | `mapping_failed`, `retokenization_failed`, `injection_failed`, `nlp_no_sentences` |

The `fa:`, `monotonicity:`, and `utr:` decisions are emitted by the
FA pipeline via `inject_review_tiers` in
`crates/batchalign/src/chat_ops/fa/review_tiers.rs`, called from
`crates/batchalign/src/fa/incremental.rs` (and `fa/mod.rs`). `fa:` and
`monotonicity:` strategy decisions are constructed in
`crates/batchalign/src/chat_ops/fa/orchestrate.rs`; `utr:` strategies
are constructed in `crates/batchalign/src/chat_ops/fa/utr.rs`. The
generalized cross-pipeline writer `inject_decision_tiers`
(`crates/batchalign-transform/src/decisions.rs`) is also called by
morphotag's incremental path (`morphosyntax/mod.rs`), which is how
`morphosyntax:` decisions reach the `%xalign` tier. All call sites take a
`ReviewLevel`, which defaults to `None`, so nothing is written unless a
caller opts in.

## Controlling emission: review level

Both the `align` and `morphotag` commands accept `--review-level`
(`AlignArgs.review_level` / `MorphotagArgs.review_level` in
`crates/batchalign/src/cli/args/commands.rs`, mapped through
`resolve_review_level`). Daemon / programmatic callers can equivalently set
the `review_level` field in the submitted job options. Both default to
`none`.

| Level | Emits |
|---|---|
| `none` (default) | No `%xalign` or `%xrev` tiers. This is the default for both `align` and `morphotag`. |
| `low-confidence` | `%xalign` + `%xrev: [?]` only on uncertain decisions. |
| `all` | `%xalign` on every bulleted utterance plus `%xrev: [?]` on uncertain ones. |

Source: `ReviewLevel` in `crates/batchalign-transform/src/decisions.rs` and
`CliReviewLevel` in `crates/batchalign/src/cli/args/commands.rs`. Both
enums default to `None`.

## The review loop

The review loop is a **short same-cycle** process: emit, review, harvest
ratings into a training CSV, strip the tiers. Ratings that sit around
unharvested for months lose their value because the algorithm they rated
moves on.

```mermaid
flowchart TD
    align["align --review-level low-confidence|all\nemits fresh %xalign + %xrev: [?]\n(off by default)"]
    stale{"File mtime > @Comment ts?\nAlgo version differs?"}
    rerun["Rerun align\nto get fresh tiers"]
    review["Reviewer inspects\n%xrev: [?] markers"]
    rate["Reviewer replaces [?]\nwith [ok] / [early] / [late] /\n[wrong] / [corrected] / [stamped]"]
    fix["Reviewer corrects\n* tier bullets where wrong"]
    harvest["harvest_reviews\n(roadmap)\nsweeps ratings into CSV"]
    finalize["batchalign3 finalize\n(roadmap)\nstrips %x* tiers in place"]
    retrain["Algorithm tuning\n(FA weights, Stanza rules)"]
    fewer["Next run emits\nfewer [?] markers"]

    align --> stale
    stale -->|"yes — stale"| rerun
    stale -->|"no — fresh"| review
    rerun --> review
    review --> rate
    rate -->|"[early] / [late] /\n[wrong]"| fix
    fix --> rate
    rate -->|"all markers resolved"| harvest
    harvest --> finalize
    finalize --> retrain
    retrain --> fewer
    fewer --> align
```

Source files verified against:
`crates/batchalign/src/chat_ops/fa/review_tiers.rs` (writer side —
`inject_review_tiers` builds the `%xalign`/`%xrev` dependent tiers),
`crates/batchalign-transform/src/decisions.rs` (typed `DecisionStrategy`
enum, `xalign_content` rendering, and `strip_decision_tiers` cleanup
helper), and `docs/pipeline-decision-metadata-design.md` (design doc).

The `finalize` and `harvest_reviews` stages are currently **roadmap** —
they are not yet shipped. Today, reviewers clean a file by re-running with
`--review-level=none` (or by deleting the tiers in an editor). See
*Publishing a clean copy* below.

## Reviewer workflow (step by step)

1. **Open the CHAT file** in your editor of choice. Any editor that preserves
   tabs between the tier label and the content will work.
2. **Search for `%xrev: [?]`**. Each match is one decision awaiting review.
3. **Read the paired `%xalign` line**. The `module:strategy` prefix and the
   `key=value` reason string together tell you what the pipeline did.
4. **Inspect the `*` tier and the bullet**. Decide whether the decision was
   right.
5. **Replace `[?]` with a marker from the vocabulary.** If you had to fix a
   bullet, use `[corrected]` and add a short note.
6. **Save.** The file is now reviewed for that utterance.

When every `%xrev: [?]` has been replaced with a resolved marker, the file is
ready for publication.

## Publishing a clean copy

### Today

Because `none` is the default, a plain run already produces a file with no
review scaffolding:

```bash
batchalign3 --no-open-dashboard align FILE.cha -o published/ --lang eng
```

If the file already carries review tiers from an earlier opted-in run,
remove them by deleting every `%xalign` / `%xrev` line by hand, or re-run
**with review tiers enabled** (`--review-level low-confidence`), which
strips the old set and emits a fresh one. A run at `--review-level none`
does **not** strip pre-existing tiers (the injector returns early before
the strip step); it only avoids adding new ones.

**Caveat:** editing or re-running throws away any reviewed ratings; they
exist only in the file you started from. Save a copy first if you want to
preserve reviewer work.

### Roadmap

`batchalign3 finalize` will be the principled replacement:

```bash
# sketch — not yet shipped
batchalign3 finalize FILE.cha --require-reviewed
```

`finalize` will (a) refuse to run if any `%xrev: [?]` markers remain, (b)
refuse to run if the tiers are stale (file mtime newer than the align
`@Comment` timestamp, or algorithm version tag doesn't match current), (c)
strip `%x*` tiers from the file in place, and (d) rewrite the `@Comment`
provenance line to record the finalize step. No archive directory: the
persistent artifact of review is the harvested training CSV, not a
per-file archive.

**Harvest before finalize.** Once the harvest tool ships, run it on the
reviewed file to capture ratings into the training CSV, then run finalize
to strip the tiers. If you finalize without harvesting, the ratings are
lost — that's intentional; stale ratings are worthless, so the tooling
refuses to let you preserve them into an ambiguous future.

## Patterns worth attention

These are the decisions that most often warrant reviewer correction. Skim
for them first.

### Alignment (`align`)

| Prefix:strategy | What it means | What to check |
|---|---|---|
| `monotonicity:end_clamped` | Utterance end was pulled in to prevent overlap with the next utterance. | Is the overlap a real overlap (`[<]`/`[>]` style) or a timing error? |
| `monotonicity:timing_stripped` | Utterance timing was removed because it started before the previous utterance. | Was the timing just swapped? Is this a two-track conversation? |
| `utr:unmatched` | Untimed utterance could not be matched to any ASR tokens. | Is the transcript different from what was said? Is the audio silent at that point? |
| `fa:words_timing_dropped` | Word-level timings were dropped because clamping made `start >= end`. | Is the utterance bullet too narrow? Does it need widening? |
| `fa:gap_filled` | A bullet gap was filled by extending the adjacent utterance. | Did the speaker really continue, or is there silence that should stay? |

### Morphotag (`morphotag`): incremental path, opt-in only

The strategy enum (`MorphosyntaxStrategy` in
`crates/batchalign-transform/src/decisions.rs`) defines these variants, and
`DecisionRecord`s for them are constructed during morphotag injection.
They reach the `%xalign` tier only on morphotag's **incremental
reprocessing path** (`process_morphosyntax_incremental`) and only when
that job's `review_level` is above `none` (the default). The batched /
full-file morphotag path does not inject decision tiers at all.

| Prefix:strategy | What it means | What to check |
|---|---|---|
| `morphosyntax:mapping_failed` | UD→CHAT conversion failed. No `%mor`/`%gra` was produced for this utterance. | Is there an unusual word, code-switch, or punctuation that confused the tagger? |
| `morphosyntax:retokenization_failed` | Stanza split or merged tokens in a way that couldn't be mapped back to CHAT words. | Are there contractions, MWTs, or compound words that need hand-annotation? |
| `morphosyntax:injection_failed` | Word-count mismatch between main tier and `%mor`. | Often an MWT expansion issue. |
| `morphosyntax:nlp_no_sentences` | Stanza returned an empty result. | Is the utterance content so short or unusual that Stanza produced nothing? |

## FAQ

**Why do the tiers come back when I re-run the command?**

Only if you opt in. With the default `review_level` of `none`, re-running
`align` or `morphotag` does not write `%xalign` / `%xrev` at all (both
injectors return early at `None`). When you do opt in
(`--review-level low-confidence|all` for `align`, or the `review_level`
option for `morphotag`'s incremental path), the run strips any existing
review tiers it manages and emits a fresh set (`strip_decision_tiers()` in
`crates/batchalign-transform/src/decisions.rs`, called from
`inject_review_tiers` / `inject_decision_tiers`). The ratings you entered
are discarded on such a re-run, so save a copy first if you want to keep
them.

**Can I delete the tiers by hand?**

Yes. They're ordinary CHAT dependent tiers, so deleting every `%xalign`
and `%xrev` line is a safe way to clean a single file. Note that
re-running with `--review-level=none` does **not** strip tiers an earlier
opted-in run left behind (the injector returns early at `none` before the
strip step); it only guarantees no new tiers are added. To remove existing
tiers, delete them by hand or re-run with review tiers enabled (which
strips and re-emits).

**Do my ratings actually feed into algorithm improvement?**

They will, once the harvest tool ships. The
intended discipline is short-cycle: review a file, harvest the ratings
into the training CSV, finalize the file, repeat. Ratings sitting in a
file for months without being harvested **lose their value** because the
algorithm that rated them moves on. Rate the file *and* make sure someone
harvests it soon after.

**I opened a file and the `%xalign` tiers look old. Are they still useful?**

Probably not. Check the `@Comment: [ba3 align | ... | <timestamp>]` line
and compare it against the file's last modification time. If the file has
been edited since align ran, or the algorithm has been bumped since then,
the tiers are **stale** — the `%xalign` reasoning no longer reflects what
the current pipeline would do. **Rerun align before reviewing.** Stale
tiers are misleading, not helpful.

**I reviewed a file, but nobody harvested it for three months. What now?**

The ratings are probably no longer usable because the algorithm has
changed. Rerun align (which strips the old tiers and emits fresh ones),
re-review, and harvest promptly this time. Old ratings against an old
algorithm are not rescuable — they don't describe decisions the current
pipeline makes.

**What's the difference between `[wrong]` and `[corrected]`?**

`[wrong]` means the decision was wrong and the reviewer has not fixed the
`*` tier. `[corrected]` means the decision was wrong and the reviewer has
fixed the `*` tier (usually by moving a bullet or replacing a word). Use
`[corrected]` whenever you actually edit the file; use `[wrong]` only when
flagging for someone else to fix.

**Is there a `%xmor` tier?**

Not currently. Morphotag decisions are recorded in the unified `%xalign`
tier with a `morphosyntax:` prefix. A draft design in
`docs/pipeline-decision-metadata-design.md` proposes splitting per-task
tiers, but implementation depends on project-level decisions about whether
the split is worth the extra tier noise.

## Related

- [Processing Provenance](./provenance.md) — `@Comment` headers that record
  which commands were run on a file.
- [Developer: Decision Provenance](../developer/decision-provenance.md) —
  internals of the `DecisionRecord` pipeline that produces `%xalign`
  content.
