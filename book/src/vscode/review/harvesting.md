# Harvesting Review Results

**Status:** Current
**Last updated:** 2026-05-21 13:30 EDT

After reviewers have rated utterances in aligned CHAT files, the
ratings need to be collected for analysis. The `%xrev` and `%xalign`
tiers are part of the CHAT files themselves, so harvesting is just a
matter of extracting those lines.

## Tier shape

Each reviewed utterance contributes two dependent-tier lines:

```text
%xalign:	boundary_averaged
%xrev:	ok
```

`%xalign` carries the alignment strategy the pipeline chose for the
bullet (`boundary_averaged`, `gap_filled`, etc.). `%xrev` carries the
human rating: `ok`, `early`, `late`, `wrong`, `?`, or `corrected`. A
corrected utterance additionally has the new timing inline in the
`%xrev` line (e.g., `corrected 1200_1450`).

## Quick extraction with shell tooling

There is no ships-with harvester binary today; the tier format is
regular enough to extract with standard tools. The two patterns below
cover most reporting needs.

### Rating distribution across a directory

```bash
# Counts of ok / early / late / wrong / ? across every .cha
rg --no-filename '^%xrev:\s*' path/to/reviewed/ \
  | awk '{print $2}' | sort | uniq -c | sort -rn
```

### Per-strategy ratings

```bash
# Paste %xalign + %xrev pairs into one stream, then tabulate.
rg --no-filename --no-line-number '^%x(align|rev):' path/to/reviewed/ \
  | paste -d' ' - - \
  | awk '{print $2, $4}' \
  | sort | uniq -c | sort -rn
```

For richer aggregations (per-file, displacement statistics on
corrected bullets, pattern detection in reviewer notes), parse the
tiers from typed JSON via `chatter to-json` rather than re-implementing
CHAT parsing in shell.

## What to do with the results

### Aggregate quality metrics

```bash
# How many rated good vs early vs late vs wrong?
cut -d, -f5 reviews.csv | sort | uniq -c | sort -rn
```

### Per-strategy breakdown

```bash
# Are boundary-averaged bullets rated differently than gap-filled?
awk -F, '{print $4, $5}' reviews.csv | sort | uniq -c | sort -rn
```

### Displacement analysis (for corrected bullets)

```bash
# Average displacement for corrected bullets
awk -F, '$5=="corrected" && $11!="" {sum+=$11; n++} END {print sum/n "ms avg"}' reviews.csv
```

### Pattern detection

Look for systematic issues in the notes column:

```bash
grep -i 'backchannel\|overlap\|speaker' reviews.csv
```

## Comparing conditions

To compare raw align vs `--bullet-repair` vs FIXBULLETS, review the
same files under each condition into separate output directories and
re-run the rating-distribution command from "Quick extraction" above
against each directory in turn. Compare the `ok` counts across
conditions.
