# Decision evidence and legacy review tiers

**Updated:** 2026-08-30 19:35 EDT

Batchalign3 records consequential pipeline decisions as structured run
evidence. It does **not** generate `%xalign` or `%xrev` tiers in CHAT output.
This is true for `align`, `morphotag`, and every value of the legacy
`--review-level` option.

The older tier projection was preliminary review scaffolding. It was rejected
for ordinary TalkBank workflows because it cluttered transcripts, became stale
after transcript or algorithm changes, and encouraged provenance to be deleted
along with presentation metadata. Current commands remove any legacy
`%xalign` and `%xrev` tiers they encounter instead of refreshing them.

## Where decision information lives

The typed `DecisionRecord` stream is the source of truth for machine choices
such as:

- forced-alignment timing repair or removal;
- monotonicity corrections;
- utterance-timing recovery failures;
- morphosyntax mapping failures; and
- segmentation decisions that need investigation.

Job traces and evidence artifacts retain these records independently of the
published `.cha` file. This separation lets experiments analyze provenance
without making temporary diagnostics part of the transcript data model.

The `needs_review` field is a machine-generated triage signal, not a calibrated
probability. A record can be useful evidence without implying that the output
is wrong.

## Legacy `--review-level`

The values `none`, `low-confidence`, and `all` remain accepted so old scripts,
stored jobs, and clients continue to deserialize. They no longer alter CHAT
output. New automation should omit the option.

This compatibility field will be removed only in a separately announced wire
or command-surface change. Code must not use it as authority to generate CHAT
tiers.

## Reviewing and publishing

Use the run's structured evidence or the project review application during an
active evaluation. Published and delivered CHAT files should contain neither
`%xalign` nor `%xrev`.

If an older transcript contains either tier, rerunning the relevant current
command strips it. Removing those tiers does not delete the current run's
structured evidence.

## Developer invariant

Decision creation and decision presentation are separate phases:

1. pipeline stages construct typed `DecisionRecord` values and trace them;
2. orchestration retains the exact records in structured evidence;
3. CHAT serialization applies the no-review-tier policy and strips legacy
   `%xalign` / `%xrev` tiers.

Tests must prove both halves: decisions survive in evidence, and no CLI, API,
or internal review-level value can synthesize the legacy CHAT tiers.
