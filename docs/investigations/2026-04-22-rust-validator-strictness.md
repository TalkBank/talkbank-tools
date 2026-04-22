# Rust Validator Strictness — Which Authority Do We Match?

**Status:** Current — decisions landed; companion record to the 2026-04-22 session
**Last updated:** 2026-04-22 19:05 EDT
**Author:** Claude (under the maintainer's direction)

## Closure note (2026-04-22 end of session)

The decisions below landed as follows:

- **"Match the CHAT manual; use CHECK as an audit signal"** ratified
  as the strictness bar. the maintainer declined a G14 CHECK-parity gate:
  "we don't want to depend on CHECK forever" — chatter becomes the
  authority over time.
- **Category A (7 fixture bugs)** fixed in commit `112b905`.
- **Category B (5 CHECK-flagged files)** resolved via TWO separate
  actions:
  - Three files (retrace, scope-markers, errors-and-replacements)
    used dead CHAT constructs (`[/?]`, `[!*]`, `[# N]`). the maintainer's
    ruling: "They should be removed from chatter grammar entirely."
    Removal landed in commit `2ddabd9`.
  - Three files (dan/deu/russian-child-narrative) carry comma-in-%mor
    (`Int,Rel`) that the maintainer confirmed is valid CHAT. CHECK
    rejects these; we accept them. Permanent CHECK-staleness,
    documented in the companion feedback memory note.
- **E544 @Media bullet-existence** validator implemented per
  the project lead's 2026-04-21 approval (commit `81a672e`). Four fixtures
  updated with `, unlinked` status.
- **Java Chatter lexer parity fixes** (one commit `2305ab4` on
  java-chatter-stable `modernize` branch): comma in %mor,
  em-dash in headers, ‡/„ in %mor, zero-lead SECONDS in CA pauses.
- **XML emitter gaps** (6+ commits): compound word decomposition,
  formType in bracketed context, extra `<g>` removal, SES + education
  projection, retrace annotation order, %sin → `<sg>/<sw>` emission,
  alignment gate in bracketed paths, TierCursors refactor.
- **Final state**: `xml_golden_parity` 94/94 (from 39/83 at session
  start), 3146/3146 workspace tests pass, 3/97 CHECK rejects
  (acknowledged stale), chatter validate passes all 97.

## Headline empirical finding

Running `the CLAN `check` binary` over every file in
`corpus/reference/` (97 files total — the "sacred 88" has grown)
produces **13 rejections (13.4%)**. Only 3 of these surfaced through
Track B XML-parity work; the other 10 were silent. There is no
existing gate that catches reference-corpus CHECK drift.

**Critical: the rejections split between genuine fixture bugs AND
CHECK staleness**. For instance, `retrace.cha`, `dan-conversation.cha`,
`deu-conversation.cha`, and `russian-child-narrative.cha` are all
rejected by CHECK for the same `Int,Rel` comma-in-%mor construct
that the maintainer confirmed is valid CHAT this session (and that we
just fixed Java to accept). So CHECK itself is out of date on that
construct.

This means **"match CHECK" is not a safe default** — it would reject
manual-valid CHAT. See revised recommendation below.

## The question that surfaced this doc

During Track B XML-emitter parity work (2026-04-22), a principled triage
emerged for "Java rejects X but Rust accepts X":

1. Run `the CLAN `check` binary` on the file.
2. If CHECK accepts → Java is buggy, fix Java.
3. If CHECK rejects → Rust is too permissive, fix Rust.

Step 3 then surfaced a deeper question: **how strict should Rust actually
be?** Three of the nine Track B candidates (`scope-markers.cha`,
`headers-time-and-types.cha`, `french-child-speech.cha`) fell into the
CHECK-rejects-but-Rust-accepts category. the maintainer noted that Rust's
grammar uses "very permissive" token definitions as an expedient
historical choice, and asked whether we should tighten.

This doc enumerates the options and recommends a bar.

## The three strictness tiers

| Tier | Authority | What passes | What fails |
|------|-----------|-------------|------------|
| **T1 Permissive (current)** | Our grammar/validator as written | Anything syntactically parsable | Only ParseErrors |
| **T2 CHECK parity** | `check` + `depfile.cut` | Anything CHECK accepts | Anything CHECK flags as error |
| **T3 Java strictness** | Java's Chat.flex + ChatParser.g + ChatWalker.g + XSD | Anything Java round-trips through XML | Everything T2 rejects + more |

CHECK is strictly weaker than Java:

- The comma-in-%mor fix this session showed Java was rejecting `pron|wer-Int,Rel-Nom-S1` while CHECK accepted the same file.
- Java's `IPA_TABLES_MINUS_VERT` character class (Chat.flex:851) is a
  300-char hand-maintained Unicode enumeration from ~2008 that has drifted
  from the modern CHAT manual.
- Java's parser expects stricter header ordering than CHECK enforces.

## Revised hierarchy of authorities

| Authority | Role | Stability |
|-----------|------|-----------|
| **CHAT manual** (`talkbank.org/0info/manuals/CHAT.html`) | Prose truth, the project lead-authored | Slow, deliberate |
| **`depfile.cut`** (``depfile.cut` (shipped with CLAN)`) | Operational symbol allow-list | Drifts; 2026-04-04 audit found gaps |
| **`check` C binary** | Enforces depfile.cut + hardcoded C rules | Drifts with depfile; some extra hardcoded rules |
| **Java `Chat.flex`** | Hand-written jflex classes from ~2008 | Drifts noticeably (comma-in-%mor fix this session) |
| **Rust grammar** | Current permissive regime | Deliberate expediency |

CHECK is NOT the ground truth. It's an audit tool. A CHECK rejection
means **either** the fixture is invalid **or** depfile.cut is stale —
you need to consult the CHAT manual to decide which.

## Recommended bar: match the CHAT manual, use CHECK as an audit signal

### Rationale

**1. Succession-aware.** A future maintainer will read the CHAT manual
and run CHECK to understand valid CHAT. They will not reverse-engineer
Java's 2,900-line Chat.flex. CHECK is the operational authority; the
CHAT manual is the prose authority; Java is an implementation artifact
that drifts from both.

**2. CHECK is data-driven.** CHECK's token/symbol rulings come from
`depfile.cut` — a documented, auditable, git-trackable data file.
Java's rulings are hardcoded regex classes requiring Maven rebuilds
and jflex arcana to evolve.

**3. Java strictness is a downstream concern.** The strictness Java
enforces beyond CHECK is there to guarantee round-trip through the
TalkBank XML schema. That is a legitimate goal but belongs in a
separate, opt-in validation pass (e.g. `chatter validate --xml-roundtrip`)
driven directly by the XSD, not by reverse-engineering Java's lexer.

**4. T3 is unstable.** Mirroring Java means inheriting its staleness. The
comma bug this session is exactly the failure mode: Java's grammar had
drifted from real CHAT data, and the "authoritative" copy would have
propagated the bug. CHECK's depfile.cut has seen more curation recently
(per 2026-04-04 spec audit work at `clan-info/lib/fixes/`).

### What T2 compliance looks like

1. **Every file in `corpus/reference/` passes both CHECK and `chatter validate`.**
   Add this to `make verify` as a new gate (G14: CHECK-parity).
2. **Every CHECK error category gets a spec in `spec/errors/`.** That's
   how T2 differences surface as test coverage. New E-codes; TDD workflow.
3. **T3 becomes an optional `--strict-xml` flag** backed by direct XSD
   validation, not by copying Java's Chat.flex.
4. **Track B XML-parity work narrows.** Files that pass CHECK but Java
   rejects are Java bugs only; files that fail CHECK are fixture problems
   or Rust permissiveness to close. The triage becomes crisp.

### What T2 does NOT mean

- **Not** a bug-for-bug clone of CHECK's messages or exit codes.
- **Not** a commitment to use CHECK's implementation. Rust keeps its own
  typed-AST validator; it just agrees with CHECK on acceptance/rejection.
- **Not** closing all existing permissiveness in one pass. This is a
  multi-session project driven by depfile.cut audit.

## Full CHECK audit results (2026-04-22, 13/97 files)

### Category A — Likely genuine fixture bugs (fix the fixture)

| File | CHECK says | Probable issue |
|---|---|---|
| `annotation/groups-phonological.cha` | "Numbers not allowed inside words" | `‹0il y a›` — `0il` looks wrong |
| `audio/french-child-speech.cha` | "Illegal char '\\'" | Bare backslash on main tier |
| `content/quotations.cha` | "`&` can't be followed by next symbol" | `&~foo` inside quotes |
| `content/separators.cha` | "Illegal char ';'" | `;` as word separator |
| `core/headers-speaker-info.cha` | "Speaker *F_A'-T not in participants" | Internal consistency |
| `tiers/coding.cha` | "Utterance delimiter expected" | `%flo:\tfluent` missing `.`/`?`/`!` |
| `ca/nonvocal-and-long-features.cha` | "Item 'xx' is illegal" | CA-context rule? — needs consult |

### Category B — Likely CHECK / depfile.cut staleness (escalate to Leonid, keep fixture)

| File | Symbol CHECK doesn't know | Status |
|---|---|---|
| `annotation/retrace.cha` | `[/?]` uncertain-retrace | CHAT manual standard |
| `annotation/scope-markers.cha` | `[!*]` scoped-stressing | Likely manual-valid |
| `languages/dan-conversation.cha` | `hvad-Int,Rel-S1` in %mor | Same comma-in-%mor case; **proven valid this session** |
| `languages/deu-conversation.cha` | `was-Int,Rel-Nom-S1` in %mor | Same |
| `audio/russian-child-narrative.cha` | `что-Int,Rel-Acc-S1` in %mor | Same |

### Category C — Needs the maintainer's manual consult

| File | Question |
|---|---|
| `core/headers-time-and-types.cha` | CHECK: `@Options` must immediately follow `@Participants`. Is that still the modern CHAT manual rule? |

## Near-term actions (this session)

### 1. Fixture surgery for Category A only (not B or C)

Seven Category A files in `corpus/reference/` currently fail CHECK for
reasons that appear to be genuine fixture bugs (not CHECK staleness).
Per-file plan:

- **`audio/french-child-speech.cha`** — contains bare `\` on main tier.
  CHECK: "Illegal character '\' found." Surgery: remove the backslash
  from that utterance or remove the utterance. Preserve the `%wor` +
  timing fixture intent (line 14) that is the file's point.

- **`core/headers-time-and-types.cha`** — CHECK: "BASIC SYNTAX ERROR"
  at line 9 (`@Types:`). Issue is likely `@Options: CA` appearing in
  wrong position. Surgery: reorder headers per CHAT manual conventions.

- **`annotation/scope-markers.cha`** — CHECK flags lines 10 and 14.
  Requires inspection to identify the exact invalid construct
  (possibly `[!*]` or its context). Surgery: replace with CHECK-valid
  equivalents while preserving the scoped-annotation coverage.

### 2. Add G14: CHECK-parity gate

Add a new gate to `make verify` that runs
`the CLAN `check` binary` over every file in
`corpus/reference/` and asserts zero errors. This prevents future
drift and makes the bar enforceable in CI.

Gate implementation sketch:

```make
.PHONY: check-parity
check-parity:
	@for f in $$(find corpus/reference -name '*.cha'); do \
	  cd $$(dirname $$f); \
	  the CLAN `check` binary $$(basename $$f) 2>&1 \
	    | grep -q 'ALL FILES CHECKED OUT OK' || exit 1; \
	  cd - > /dev/null; \
	done
	@echo "G14 ✓ CHECK parity"
```

(Real implementation would batch; this is illustrative.)

### 3. Record the tiebreaker rule in CLAUDE.md

The feedback memory note I saved today (`feedback_check_is_tiebreaker.md`)
should graduate to a CLAUDE.md rule so every contributor sees it, not
just my future sessions.

## Longer-term work (multi-session)

### Phase 1 — Baseline audit (1–2 sessions)

- Run CHECK over the full external corpus set.
- Tabulate error categories CHECK reports that Rust currently accepts.
- One E-code per category, with spec/corpus/test.

### Phase 2 — Data-driven acceptance (2–3 sessions)

- Read `clan-info/lib/depfile.cut` into the Rust validator directly.
- Replace hand-written character classes with depfile-driven ones
  where applicable.
- Each depfile entry becomes a typed newtype boundary.

### Phase 3 — XSD-driven strict mode (optional, 1 session)

- Implement `chatter validate --xml-roundtrip` using the TalkBank XSD
  directly (we already have `talkbank-xml-schema 3.2.2` as a dep).
- This covers T3 concerns without inheriting Java's staleness.

### Out of scope

- Copying Java's `IPA_TABLES_MINUS_VERT` verbatim. If specific IPA
  characters need handling, source from Unicode databases + the CHAT
  manual's %pho section, not Java.
- Forcing Java-style header ordering into the default validator. Java's
  stricter ordering rules are arguably bugs in Java (the modern CHAT
  manual permits the orderings CHECK allows).

## Open questions

1. Is there a definitive list of CHECK error categories, or do we have
   to discover them corpus-by-corpus? (Probably the latter. Leonid's
   C source could be mined if needed.)
2. Does `depfile.cut` drift from what CHECK actually enforces? (Spec
   audit on 2026-04-04 found 51/243 specs broken — see
   `project_spec_audit` memory note. Similar audit of depfile.cut
   against CHECK C source would be worthwhile.)
3. Do we want a `--permissive` escape hatch for incoming corpus data
   that fails CHECK but we still need to accept (with warnings)?

## Decision requested

- [ ] Endorse **"match the CHAT manual; use CHECK as an audit signal"**
      (not raw CHECK parity) as the default strictness bar.
- [ ] Authorize fixture surgery on the ~7 Category A files this session.
- [ ] Authorize a letter to Leonid with the Category B files, asking for
      a depfile.cut update (or clarification that CHECK's C code rejects
      these independent of depfile).
- [ ] Rule on Category C (`@Options` ordering).
- [ ] Authorize adding G14 gate: every `corpus/reference/` file must pass
      both `chatter validate` (default permissive) AND — for files not
      on a documented Category B exemption list — `check`.
- [ ] Commit to Phase 1 (broader baseline audit across external corpora)
      as a future session.
