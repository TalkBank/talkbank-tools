# CLAN parity testing

Parity-test scripts for verifying `chatter clan <cmd>` against the
original CLAN binary at `OSX-CLAN/src/unix/bin/<cmd>`. Treat the
OSX-CLAN binary as the **authoritative reference**: same posture as
the discrepancy-adjudication policy uses for `CHECK`.

## Build OSX-CLAN

The makefile at `OSX-CLAN/src/clan/makefile` ships with all CFLAGS
lines commented out so the platform pick stays explicit. Build a
specific command via CFLAGS override (no makefile edit needed) on
macOS Big Sur and later:

```bash
cd OSX-CLAN/src/clan
make -B CC=g++ \
  CFLAGS='-O -DUNX -DAPPLEUNX -fshort-wchar -Wcomment \
          -Wno-deprecated-declarations -Wempty-body \
          -Wno-c++11-compat-deprecated-writable-strings' \
  freq mlu kwal
```

`-B` forces a clean rebuild so the binary reflects the current source.
Adapt the CFLAGS line per the platform comments in the makefile.

Compiled binaries land at `OSX-CLAN/src/unix/bin/<cmd>`.

## Run one parity test

```bash
scripts/clan-parity/run-parity.sh <cmd> <fixture.cha> [extra_args...]
```

The script writes
`scripts/clan-parity/out/<cmd>.{clan,chatter}.txt` and prints a
side-by-side `diff -u`. Outputs are gitignored — re-run to refresh.

**Invocation convention.** CLAN analysis commands take input via
**stdin redirection** (`<`), not as a positional file argument. The
script handles this. For chatter, the positional file argument is
used.

## Interpreting the diff

**Presentation parity is required.** The TalkBank mandate is to
reproduce CLAN tools faithfully in input and output. Researchers'
eyes are trained on the exact CLAN output format; a
semantically-equivalent but presentation-divergent output is not
parity, it's a different tool. Sort order, headers, banners,
advisory text, indentation, section dividers, the leading "From pipe
input" + command-echo line — all of those count.

For each diff, classify the outcome into one of three buckets:

1. **Match** — the diff is empty, or contains only date-of-invocation
   text that necessarily varies between runs. Record as "CLAN parity:
   matches" on the command's clan-reference page.
2. **Documented intentional divergence** — reserved for **deliberate
   scope decisions** like the UD-vs-CLAN-MOR architectural choice
   (whole commands deliberately not implemented; see `mor.md` /
   `megrasp.md` / `post.md` / `postlist.md` / `postmodrules.md` /
   `posttrain.md`). Do not invent new "intentional divergences" for
   presentation differences — those go in bucket 3.
3. **Real gap** — every other difference. Examples: chatter sorting
   frequency-desc when CLAN sorts alphabetically; chatter omitting
   CLAN's date-banner header; chatter omitting CLAN's TTR advisory
   text; chatter adding a min-max range that CLAN does not. Add an
   `Implementation status` row on the clan-reference page and queue
   the implementation work.

Examples vetted so far (semantic computation matches; presentation
needs work):

- **freq**: types / tokens / TTR / per-speaker breakdowns match
  semantically. Real gaps: sort order (CLAN alphabetical, chatter
  frequency-desc); missing date-banner header line; missing
  "From pipe input" + command-echo line; missing TTR advisory text
  ("This TTR number was not calculated on the basis of %mor line
  forms. If you want a TTR based on lemmas, run FREQ on the %mor
  line with option: +sm;*,o%"); hyphen section dividers.
- **mlu**: utterances / morphemes / MLU ratio / SD match
  semantically. Real gaps: missing date-banner; missing "MLU (xxx,
  yyy and www are EXCLUDED from the utterance and morpheme counts)"
  explainer; chatter emits an extra "Range: 1-6" line that CLAN does
  not.

## Fixtures

Use small per-command fixtures (5–10 utterances) so the diff stays
readable. Real-corpus testing comes after the basic shape parity is
established.

Suggested locations:

- ad-hoc fixtures: `/tmp/parity-<cmd>.cha` (don't commit)
- canonical fixtures (once stable): `tests/fixtures/clan-parity/<cmd>.cha`

## Scaling pattern

Process commands in priority order:

1. Highest-traffic analysis: `freq`, `mlu`, `mlt`, `kwal`, `combo`,
   `freqpos`, `vocd`.
2. Profiling and assessment: `dss`, `ipsyn`, `kideval`, `eval`,
   `eval-d`, `sugar`.
3. Transforms (smaller divergence surface): `flo`, `dataclean`,
   `compound`, `delim`, etc.
4. Format converters: `chat2*`, `*2chat`.

The 6 MOR/POST stubs (mor, megrasp, post, postlist, postmodrules,
posttrain) are intentionally not implemented — skip them.

## What "real gap" triggers

A real-gap classification means **implement the fix in
`chatter clan`**, not document the divergence. Doc updates follow
the implementation: once the diff is empty, the command page
records "CLAN parity: matches" and the DRAFT `Display Modes
(+dN / --display-mode N)` section on that page can be re-evaluated
(resolved if the `+dN` flag mapping is now in scope; explicitly
deferred otherwise).

Implementation lives next to each command in
`crates/talkbank-clan/src/commands/<cmd>.rs`. The output
formatting layer in `crates/talkbank-clan/src/framework/render*`
is where the bulk of the presentation-parity work will land —
headers, banners, sort-order, advisory text, section dividers.
