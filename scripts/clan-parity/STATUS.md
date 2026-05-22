# CLAN parity status

Per-command tracking of `chatter clan <cmd>` vs
`OSX-CLAN/src/unix/bin/<cmd>` byte-level diff under
[`scripts/clan-parity/run-parity.sh`](run-parity.sh).

Three intrinsic differences are excluded from every status below:

1. **Timestamp** — `ctime`-style date string differs per invocation.
2. **Version string** — `(27-Mar-2026)` build date in CLAN vs
   `(0.2.0)` semver in chatter.
3. **Source line** — CLAN says `From pipe input` (it can only read
   stdin), chatter says `From file <basename>`.

CLAN also emits its own startup banner **twice** in stdin
invocations (the `FirstTime` branch in `cutt.cpp`). chatter emits
the banner once.

## Body byte-level parity

These commands produce CLAN-format output character-for-character
identical to the legacy binary (modulo the three intrinsics + the
banner-duplication quirk above):

| Command | Last fix |
|---|---|
| **freq** | `e7f2cc4b` |
| **mlu** | `6dce43ed` |
| **mlt** | `25a91983` |
| **freqpos** | `25a91983` |
| **kwal** | `25a91983` (no per-command fix; landed via scope refactor) |
| **combo** | `40c71003` |
| **wdlen** | `ae42f3f0` |
| **vocd** | `ae42f3f0` (lemma echo matches; warning section still missing) |
| **phonfreq** | `10bde264` |
| **dist** | `10bde264` |
| **maxwd** | `067992bf` (scope reclassified to MainOnly) |
| **eval** | `067992bf` (refusal byte-identical) |
| **eval-d** | `067992bf` (refusal byte-identical) |
| **sugar** | `067992bf` (refusal byte-identical) |
| **kideval** | `067992bf` (refusal byte-identical; preserves CLAN's leading blank line) |

Plus the four required-flag refusals — `chains`, `dss`, `ipsyn`,
`mortable` — emit byte-identical stderr messages to CLAN's
pre-banner refusal (commit `7ae7ef76`). chatter exits with code 1
and no banner, exactly matching CLAN.

## Partial parity / open work

| Command | Open gap |
|---|---|
| **vocd** | `*** File "pipeout": Speaker: "*X:" \n vocd: WARNING: Not enough tokens for random sampling without replacement.` pre-banner warning block when sample size is below the bootstrap threshold. chatter currently runs silently. |
| **codes** | Banner is missing two scope-continuation lines: `and those speakers' ONLY dependent tiers matching: %COD:;` and `and ONLY header tiers matching: @ID:;`. The `@ID:` header-filter continuation is a *fourth* banner shape on top of the three already modelled in `ClanScopeMode`. |
| **wdsize**, **complexity** | CLAN writes the analysis body to a **sidecar file** (`pipeout.wdsize.cex`, `pipeout.complexity.xls`), not stdout. CLAN's stdout shows the banner + a single `Output file <pipeout.<cmd>.<ext>>` line. chatter writes the analysis to stdout. Implementing the sidecar-writing pattern is the open work for this class of command. |
| **complexity** | The 4th-banner-shape `and ONLY header tiers matching: @ID:;` continuation is missing. Same gap as codes. Scope mode currently sets `%GRA:;` correctly but the header-filter continuation isn't modelled in `ClanScopeMode`. |
| **kwal**, **combo** | CLAN echoes the invocation args in the banner first line (`kwal +scat`, `combo +sthe^cat`); chatter shows just the command name. Cross-cutting — applies to every command. |

## Required-flag refusal (byte-identical to CLAN)

| Command | Required flag | CLAN error message |
|---|---|---|
| `chains` | `--tier` | `Please specify a code tier with "+t" option.` |
| `dss` | `--speaker` (at least one) | `Please specify at least one speaker tier name with "+t" option.` |
| `ipsyn` | `--rules` | Two-line message ending with `For example, "ipsyn +leng"…` |
| `mortable` | `--script` | Two-line message ending with `For example, "mortable +leng"…` |
| `eval` | `--speaker` (at least one) | `…speaker tier code with "+t" option on command line.` |
| `eval-d` | `--speaker` (at least one) | Same wording as eval |
| `sugar` | `--speaker` (at least one) | Same wording as eval |
| `kideval` | `--dss-rules` and/or `--ipsyn-rules` | Two-line with leading blank line |

## Out of scope / deliberately not implemented

The CLAN MOR/POST grammar family (`mor`, `megrasp`, `post`,
`postlist`, `postmodrules`, `posttrain`) is deliberately not
implemented in talkbank-tools; see the per-command stub pages under
`book/src/clan-reference/commands/`. Skip these in any parity sweep.

## Cross-cutting follow-up

- **Banner duplication.** CLAN emits its banner twice per stdin
  invocation; chatter elides the duplicate. Not user-facing
  meaningful but blocks byte-level diff.
- **Version-string shape.** chatter's `(0.2.0)` semver vs CLAN's
  `(DD-Mon-YYYY)` build date.
- **Invocation-args echo.** CLAN's banner first line includes the
  CLI flags (`freq +scat`); chatter shows just the command name.
- **Speaker-tier filtering refinements.** `ALL speaker tiers with
  IDs`, `… EXCEPT the ones matching: …` — modelled as TODOs in
  `helpers.rs::clan_scope_for`.
- **Header-tier filter continuation.** Codes-class commands emit
  `and ONLY header tiers matching: @ID:;` after the dependent-tier
  line — a fourth banner shape on top of the three in
  `ClanScopeMode`.
- **Strict no-flag refusal.** chains, dss, ipsyn, mortable refuse
  to run in CLAN when their required flag is absent. chatter
  produces output instead. Match CLAN's strictness or accept the
  divergence as a chatter-side improvement.

## Build notes

Rebuild OSX-CLAN binaries on macOS Big Sur+ with:

```bash
cd ~/talkbank/OSX-CLAN/src/clan
make -B CC=g++ \
  CFLAGS='-O -DUNX -DAPPLEUNX -fshort-wchar -Wcomment \
          -Wno-deprecated-declarations -Wempty-body \
          -Wno-c++11-compat-deprecated-writable-strings' \
  freq mlu mlt kwal combo freqpos vocd wdlen phonfreq \
  dist codes chains dss ipsyn mortable maxwd \
  kideval eval eval-d sugar
```

`wdsize`, `complexity`, `corelex` have **no explicit `$(DD)/<cmd>`
rule in the OSX-CLAN makefile**, so they hit make's implicit `c++
<cmd>.cpp -o <cmd>` rule, which drops the CFLAGS and fails on
`unCH` header-type errors. Workaround: build them by hand with the
full g++ command line:

```bash
cd ~/talkbank/OSX-CLAN/src/clan
for cmd in wdsize complexity; do
  g++ -O -DUNX -DAPPLEUNX -fshort-wchar -Wcomment \
      -Wno-deprecated-declarations -Wempty-body \
      -Wno-c++11-compat-deprecated-writable-strings \
      $cmd.cpp -o ../unix/bin/$cmd \
      ../unix/obj/c_ver.o ../unix/obj/cut.o ../unix/obj/stp.o \
      ../unix/obj/fntc.o
done
```

`corelex` requires extra source files (`isMatch`/`ALTLABELS` symbols
from another translation unit); needs the right object set to
build. Investigate when corelex is in scope.
