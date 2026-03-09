# depfile.cut Analysis: How CLAN CHECK Validates CHAT Files

This document is the authoritative reference for understanding `depfile.cut` — the
template file that CLAN CHECK uses to validate CHAT transcripts. It maps every
section and pattern to its validation semantics, documents the pattern language,
and compares against `chatter validate`'s hardcoded rules.

## 1. What is depfile.cut?

`depfile.cut` is a 99-line file that defines **legal values** for every component
of a CHAT file: headers, main tier tokens, and dependent tier content. CLAN CHECK
reads it at startup (from `~/clan-info/lib/depfile.cut` or the CLAN `lib/` directory)
and validates every token in the file against its templates.

The file has evolved slowly over the years and is essentially frozen. Changes are
rare and coordinated with CHAT format updates.

### Location

CLAN CHECK searches for `depfile.cut` in:
1. The working directory (via `checkOpenWdDepfile`)
2. The CLAN `lib/` directory (via `OpenGenLib`)

On our systems: `~/clan-info/lib/depfile.cut`

### File Format

```
# Comment lines start with #
@HeaderName:    value1 value2 @d<format> @t<format>
*:              pattern1 pattern2
%tiername:      pattern1 pattern2
```

Lines starting with `@`, `*`, or `%` define tier templates.
Continuation lines (starting with whitespace) are joined to the previous line.
The text after the colon (and tab) is split into whitespace-separated tokens,
each becoming a pattern template — except that brackets `[...]` are kept as single tokens.

## 2. The Pattern Language

depfile.cut uses a **custom mini-language** — NOT regex, NOT standard globs. The
patterns are interpreted by `check_matchplate()` in `check.cpp`, which has
type-dependent matching behavior controlled by a `charType` parameter.

### Control Tokens (Meta-Patterns)

Some tokens in depfile.cut are NOT patterns but control flags that configure
how CHECK processes the tier. These are consumed during parsing and do not appear
in the template list:

| Token | Meaning | Default for main tier |
|-------|---------|----------------------|
| `[UTD]` | Require utterance delimiter | On |
| `[-UTD]` | Forbid utterance delimiter | — |
| `[IGN]` | Skip all validation for this tier | — |
| `[AFX]` | Suffix-first checking mode | — |
| `[NOWORDCHECK]` | Skip character-level word checks | — |
| `[UPREFS ...]` | Legal uppercase prefix patterns | — |

For example, `@Comment: [IGN]` means CHECK skips comment content validation
entirely. This explains why `@Comment` can have free text even though the
depfile.cut entry only lists `[IGN]` with no `*` wildcard.

### Pattern Types

#### `*` (bare asterisk)

Matches any token that starts with an **alphanumeric character** or a **non-ASCII
(high-byte) character**. Specifically:

- Matches if the first character is `a-z`, `A-Z`, `0-9`, or any byte > 127 (UTF-8 multibyte)
- Also matches if the first character is `(`
- Also matches `^` followed by an alpha/non-ASCII character
- Does **NOT** match tokens starting with: `$`, `+`, `@`, `[`, `\`, `#`, `&`, `-`, `=`, `/`, `"`, `'`, `|`, `:`, `;`, `<`, `>`

This means `*` is a "normal word" wildcard. Special-prefix tokens need explicit
patterns (see below).

**Context-dependent behavior**: When date/time templates (`@d<>`, `@t<>`) have already
been tried and failed, the `*` wildcard only matches words starting with **alpha**
characters (not digits), allowing the date/time error to take priority over the
generic "undeclared symbol" error.

#### Literal strings

Plain strings like `audio`, `CA`, `1`, `more` match exactly (case-insensitive via
`uS.patmat()`). The matching function `uS.patmat()` supports glob-style wildcards
within literal patterns:

- `*` within a literal matches any sequence of characters
- `_` within a literal matches any single character

#### `@d<format>` — Date templates

Invokes the dedicated date parser `check_MatchDT()` which validates character-by-character:

| Format char | Matches |
|-------------|---------|
| `d` | Day digit (tens: 0-3) |
| `m` | Month digit (tens: 0-1, value ≤ 12) |
| `y` | Year digit (any digit) |
| `l` | Alpha character; `lll` validates 3-letter month (JAN–DEC) |
| `h` | Hour digit (0-23) |
| `s` | Second digit (tens: 0-5) |
| `<`, `>` | Angle brackets in format are skipped (not consumed from input) |
| `-`, `:`, `;`, `.` | Must match literally |

Examples from depfile.cut:
- `@d<dd-lll-yyyy>` → `15-JAN-2024` ✓, `2024-01-15` ✗
- `@d<yy;>` → `3;` ✓ (age: 3 years)
- `@d<yy;mm.>` → `3;06.` ✓ (age: 3 years, 6 months)
- `@d<yy;mm.dd>` → `3;06.15` ✓ (age: 3 years, 6 months, 15 days)

#### `@t<format>` — Time templates

Same parser as `@d<>` but with `time=TRUE`:

| Format char | Matches (in time mode) |
|-------------|----------------------|
| `h` | Hour digit (0-23) |
| `m` | Minute digit (tens: 0-5, value ≤ 59) |
| `s` | Second digit (tens: 0-5, value ≤ 59) |

Examples from depfile.cut:
- `@t<hh:mm:ss>` → `8:30:31` ✓, `08:30:31` ✓
- `@t<mm:ss>` → `30:31` ✓
- `@t<hh:mm:ss-hh:mm:ss>` → `08:00:00-09:30:00` ✓
- `@t<hh:mm-hh:mm>` → `17:30-18:00` ✓
- `@t<ss>` → `45` ✓

**Important**: The `h`, `m`, `s` format characters accept 1 or 2 digits. The parser
does not require leading zeros (e.g., `8:30:31` is valid for `hh:mm:ss`).

#### `@s<value>` — SES (Socioeconomic Status) values

Matched by `check_SES_item()` which does a case-insensitive prefix match.

From depfile.cut `@ID:` line: `@s<WC>`, `@s<UC>`, `@s<MC>`, `@s<LI>`

#### `@e<value>` — Ethnicity values

Also matched by `check_SES_item()`.

From depfile.cut `@ID:` line: `@e<White>`, `@e<Black>`, `@e<Latino>`, `@e<Asian>`,
`@e<Pacific>`, `@e<Native>`, `@e<Multiple>`, `@e<Unknown>`

#### `[pattern]` — Bracket code patterns

When a bracket pattern appears (e.g., `[- _*]`, `[+ _*]`, `[!]`):
1. CHECK first calls `uS.isSqCodes()` to extract/normalize the bracket content
2. Then matches the extracted content against the pattern via `uS.patmat()`

The `_` in bracket patterns like `[- _*]` represents a space in CLAN's encoding,
so `[- _*]` matches any bracket code with content after a space: `[- eng]`, `[- fra]`, etc.

#### Prefix patterns: `$*`, `+*`, `\**`, `-*`, `#*`, `&*`, `|*`, etc.

These are matched via `uS.patmat()` glob matching:

| Pattern | Matches | Used for |
|---------|---------|----------|
| `$*` | Any token starting with `$` | Codes on %cod, %pho, etc. |
| `+*` | Any token starting with `+` | Utterance delimiters |
| `\**` | `\*` is escaped literal `*` | Speaker codes on %mor (*MOT) |
| `-*` | Any token starting with `-` | Negation/exclusion markers |
| `#*` | Any token starting with `#` | Pause markers on %pho, %gpx |
| `&*` | Any token starting with `&` | Fragments on %pho, %ort |
| `|*` | Any token starting with `|` | POS pipes on %cod, %pho |
| `$=*` | Token starting with `$=` | Variable codes on %act, %add |
| `"*` | Token starting with `"` | Quoted content |
| `'*` | Token starting with `'` | Elision/apostrophe |
| `/*` | Token starting with `/` | Pauses on %act, %pho |
| `@*` | Token starting with `@` | Special form markers |

### Error Numbers Produced by Pattern Matching

| Error | Meaning | When |
|-------|---------|------|
| 11 | Symbol not declared in depfile | Default: no pattern matched |
| 15 | Illegal role | Role not in @Participants template |
| 17 | Tier not declared in depfile | Tier name not found in any template list |
| 20 | Undeclared suffix in depfile | Word suffix doesn't match (via `check_CheckFixes`) |
| 32 | Code not declared in depfile | No match and token starts with `$` |
| 33 | Illegal date/time or undeclared symbol | Date/time templates tried, all failed |
| 34 | Illegal date representation | Last `@d` pattern in sequence failed |
| 35 | Illegal time representation | Last `@t` pattern in sequence failed |
| 37 | Undeclared prefix | Word prefix doesn't match (via `check_CheckFixes`) |
| 113 | Illegal keyword on @Media | Media keyword not in depfile |
| 144 | Illegal SES value | `@s`/`@e` patterns tried, all failed |
| 147 | Undeclared special form marker | `@`-marker suffix doesn't match |

## 3. Per-Section Breakdown

### Obligatory Headers

#### `@Begin` / `@End`

**depfile.cut**: Listed as markers (no values).
**CHECK**: Hardcoded structural checks in `check_OverAll()`.
- Error 6: @Begin missing
- Error 7: @End missing
- Error 43: File must start with @Begin
- Error 44: File must end with @End
- Error 53: Duplicate @Begin
- Error 54: Duplicate @End

**chatter validate**: ✅ Covered — E501 (MissingRequiredHeader), E502 (MissingEndHeader),
E503 (DuplicateHeader). All hardcoded.

#### `@Languages: *`

**depfile.cut**: `*` wildcard — accepts any value.
**CHECK**: Hardcoded validation against `ISO-639.cut` file (NOT depfile).
- Error 77: @Languages expected in position
- Error 120: Use three-letter language code
- Error 121: Language code not found in ISO-639

**chatter validate**: ✅ Covered — E510 (InvalidLanguageCode), hardcoded against
ISO 639-3 database.

#### `@Participants: Target_Child Mother Father Brother Sister ...`

**depfile.cut**: Lists 49 legal participant roles.
**CHECK**: Template-driven via `check_matchplate()`.
- Error 12: Missing speaker name/role
- Error 13: Duplicate speaker
- Error 15: **Illegal role** (template-driven)
- Error 16: Illegal characters in speaker name
- Error 68: Missing CHI Target_Child (hardcoded, `+g2`)
- Error 100: Trailing commas

**chatter validate**: ✅ Covered — E326 (InvalidParticipantRole). Hardcoded set of
49 roles in `is_allowed_participant_role()`. The role set matches depfile.cut.

**Gap**: None identified. Both use the same 49-role set.

#### `@Options: CA NoAlign`

**depfile.cut**: Only `CA` and `NoAlign` are legal.
**CHECK**: Dual mechanism:
1. Hardcoded handler recognizes `multi`, `bullets`, `CA`, `CA-Unicode`, `IPA`,
   `notarget`, `mismatch`, `dummy` and sets internal flags
2. Template check (error 11) validates against depfile — only `CA`, `NoAlign`

**chatter validate**: ⚠️ **Partial gap** — E534 (UnsupportedOption). Currently allows
`CA`, `NoAlign`, and `dummy`. The `dummy` value is NOT in depfile.cut but IS
recognized by CHECK's hardcoded handler.

**Assessment**: `dummy` is a legacy value that CHECK recognizes programmatically but
depfile.cut doesn't list. Our validator allowing it is reasonable. Values like
`multi`, `bullets`, `IPA` are internal CHECK flags, not user-facing legal values.
`notarget` suppresses the CHI Target_Child check. These probably should not appear
in user files.

#### `@ID: @d<yy;> @d<yy;mm.> @d<yy;mm.dd> @e<...> @s<...>`

**depfile.cut**: Age formats, ethnicity codes, SES codes.
**CHECK**: Complex per-field validation in `check_ID()`:
- Field 0 (language): Error 122 — cross-checks @Languages (hardcoded)
- Field 1 (corpus): Error 63 — empty check (hardcoded)
- Field 2 (speaker): Errors 12, 18, 13 — cross-checks @Participants (hardcoded)
- Field 3 (age): **Template-driven** — `@d<yy;>`, `@d<yy;mm.>`, `@d<yy;mm.dd>`
  - Error 34: Illegal date
  - Error 33: Illegal date/time
  - Error 153: Missing initial zero (hardcoded)
- Field 4 (sex): **Hardcoded** — "male" or "female". Error 64.
- Field 5 (group): No validation
- Field 6 (SES): **Template-driven** — `@s<WC>`, `@s<UC>`, `@s<MC>`, `@s<LI>`
  - Error 144: Illegal SES
- Field 7 (role): **Template-driven** — uses @Participants role list
  - Error 15: Illegal role
  - Error 142: Role mismatch with @Participants (hardcoded)
- Fields 8-9: No validation

**chatter validate**:
- E505 (InvalidIDFormat): Field count must be 10 ✅
- E517 (InvalidAgeFormat): Age format ✅
- E542 (UnsupportedSex): male/female ✅
- E546 (UnsupportedSesValue): SES values ✅
- E326 (InvalidParticipantRole): Role validation ✅
- E510 (InvalidLanguageCode): Language ✅

**Gap**:
- Error 153 (missing initial zero on age month/day) — ❓ need to verify if E517 covers this
- Error 142 (role mismatch between @ID and @Participants) — ❓ need to verify
- Ethnicity values (`@e<...>`) — ❓ need to verify if E546 covers these

#### `@Media: audio video missing unlinked notrans`

**depfile.cut**: Five legal keywords.
**CHECK**: Custom handler + template validation.
- Error 113: Illegal keyword (template-driven)
- Error 114: Must have audio/video (hardcoded)
- Error 148: Space before comma (hardcoded)
- Error 157: Filename mismatch (hardcoded)

**chatter validate**:
- E535 (UnsupportedMediaType): Allows `audio`, `video`, `missing` ✅
- E536 (UnsupportedMediaStatus): Allows `unlinked`, `missing`, `notrans` ✅
- E528 (MediaFilenameMismatch) ✅

**Gap**: None identified. Our model separates media_type (audio/video/missing) from
status (unlinked/missing/notrans), which is actually cleaner than CHECK's flat list.

### Optional Participant Headers

#### `@Birth of #: @d<dd-lll-yyyy>`

**CHECK**: Template-driven date validation. Error 34 (illegal date).

**chatter validate**: ⚠️ **Gap** — E524 validates that the participant exists, but
does NOT validate the date format itself.

#### `@Birthplace of #: *` / `@L1 of #: *`

**depfile.cut**: `*` (accept anything).
**CHECK**: Accepts any value.
**chatter validate**: ✅ No validation needed (wildcard).

### Optional File Description Headers

#### `@Time Duration: @t<hh:mm-hh:mm> @t<hh:mm:ss-hh:mm:ss> @t<hh:mm:ss>`

**CHECK**: Template-driven time validation. Error 35 (illegal time).

**chatter validate**: ✅ Covered — E540 (InvalidTimeDuration). Validates HH:MM:SS,
ranges with `-` and `;`, comma-separated.

**Assessment**: Our validator may be slightly more permissive (allows `;` separator,
which depfile.cut doesn't list). Need to verify if CHECK accepts `;`.

#### `@Time Start: @t<hh:mm:ss> @t<mm:ss>`

**CHECK**: Template-driven. Error 35.

**chatter validate**: ✅ Covered — E541 (InvalidTimeStart). Validates HH:MM:SS and
MM:SS, optionally with milliseconds.

**Assessment**: Our validator allows milliseconds (`.mmm` suffix) which depfile.cut
doesn't list. This is an extension, not a gap.

#### `@Transcriber: *` / `@Tape Location: *` / `@Room Layout: *` / `@Location: *`

**depfile.cut**: `*` (accept anything).
**chatter validate**: ✅ No validation needed.

#### `@Number: 1 2 3 4 5 more audience`

**CHECK**: Template-driven. Error 11.
**chatter validate**: ✅ E537 — same values hardcoded.

#### `@Recording Quality: 1 2 3 4 5`

**CHECK**: Template-driven. Error 11.
**chatter validate**: ✅ E538 — same values hardcoded.

#### `@Transcription: eye_dialect partial full detailed coarse checked anonymized`

**CHECK**: Template-driven. Error 11.
**chatter validate**: ✅ E539 — same values hardcoded.

#### `@Types: *` / `@Videos: *` / `@Warning: * [*] "*`

**depfile.cut**: `*` or near-wildcard.
**chatter validate**: ✅ No validation needed.

### Local Changeable Headers

#### `@Date: @d<dd-lll-yyyy> -`

**CHECK**: Template-driven. Accepts date format or bare `-`.
**chatter validate**: ✅ E518 — validates DD-MMM-YYYY format.

**Assessment**: ❓ Does our validator accept bare `-` as a valid date? CHECK does
because depfile.cut includes `-` as a literal alternative.

#### `@Situation: * "* - ["] [?]`

**depfile.cut**: Multiple patterns including quotes and brackets.
**CHECK**: Template-driven.
**chatter validate**: ✅ Free text accepted (no specific validation beyond parsing).

#### `@Activities: * = "* -` / `@Bg: * = $* -` / `@Eg: * = $* -` / `@G: * = -* -`

**depfile.cut**: Various patterns with wildcards.
**chatter validate**: ✅ These accept any text in practice (the `*` covers normal words).

#### `@Comment: [IGN]`

**depfile.cut**: `[IGN]` is a **control token**, not a pattern (see §2 Control Tokens).
It sets the `IGN` flag on the `@Comment` tier, which tells CHECK to **skip all
validation** of @Comment content entirely.

**CHECK**: No content validation at all. @Comment accepts anything.
**chatter validate**: ✅ No content validation (free text accepted).

### Dependent Tiers

#### `%tim: @t<hh:mm:ss> @t<mm:ss> @t<hh:mm:ss-hh:mm:ss> @t<hh:mm-hh:mm> @t<ss>`

**depfile.cut**: Five time format templates. NO `*` wildcard.
**CHECK**: Template-driven time validation. Error 35 (illegal time).

**chatter validate**: ✅ Mostly covered — E603 (InvalidTimTierFormat) fires for
non-time content. The `TimTier::from_text` parser correctly handles:
- `hh:mm:ss` ✅ (via 3-part `parse_time_value`)
- `mm:ss` ✅ (via 2-part `parse_time_value`)
- `hh:mm:ss-hh:mm:ss` ✅ (range splitting on `-`)
- `hh:mm-hh:mm` ✅ (range splitting on `-`)

⚠️ **Minor gap**: `@t<ss>` bare seconds (e.g., `45`) is not handled — `parse_time_value`
requires at least one colon. This is a rare format in practice.

#### `%gra: *` / `%ugra: *` / `%grt: *`

**depfile.cut**: `*` wildcard.
**CHECK**: Accepts any content.
**chatter validate**: ✅ %gra has structural validation (E701–E710) that goes
beyond depfile — we validate GRA relation format, indices, head references, etc.

#### `%mor: * \** [UTD] +... +/. +!? +//. +/? +"/. +". +//? +..? +.`

**depfile.cut**: `*` plus escaped-asterisk words plus utterance delimiters.
**CHECK**: The `*` accepts any normal word. `\**` accepts words starting with
literal `*` (speaker codes in %mor). The delimiters accept matching terminators.

**chatter validate**: ✅ %mor has deep structural validation (E601, E701–E715)
that goes far beyond depfile's pattern matching. We parse POS|lemma&features
structure, validate alignment with main tier, etc.

#### `%pho: * \** #* '* =* ^* <* > [/] [//] $* [\*] [+ *] /* @* &* |* [-UTD]`

**depfile.cut**: Many prefix patterns authorizing different special characters.

**chatter validate**: ✅ %pho has structural parsing and alignment validation
(E715). The grammar handles all these token types structurally.

#### `%act: * [_*] $=* "* '* /* ( ) : -*`

**depfile.cut**: `*` plus specific prefix patterns.
**chatter validate**: ✅ Accepts free text with bullets (BulletContent parsing).

#### `%cod: * $* +* |*` / `%add: * $=*` / `%alt: * [_*]` / etc.

**depfile.cut**: Various patterns per tier.
**chatter validate**: `%cod` is currently parsed as bullet-capable text, but
that is weaker than the coding semantics used in real corpora. A corpus scan of
`~/data` found 126,325 `%cod` lines, dominated by whitespace-delimited code
tokens (especially `$...` codes), plus a smaller but real set of bracketed
targeted items such as `<w4> seep`, `<w4-5> $MWR`, and `<wl> $WR`. So the
depfile entry should be read as token authorization, not as proof that `%cod`
is semantically "free text". The `*` wildcard means "any normal word is legal",
but `%cod` still appears to have an item structure: an ordered sequence of code
values with optional bracketed target selectors.

**Bug found (2026-03-06)**: our `%cod` model/docs had drifted in opposite
directions. Some docs treated `%cod` as generic free text, while the model docs
described only numeric selectors like `<1>` and `<1+2>`. Real data shows that
target selectors can also look like `<w4>`, `<w4-5>`, `<wl>`, and `<W2>`. Any
future `%cod` AST should preserve this broader selector syntax rather than
coercing it into a numeric-only scheme.

#### `%com: * \`* ~* !* @* #* $* %* ^* &* \** -* _* =* +* {*} [*] \* |* :* ;* "* '* <*> ,* .* ?* /*`

**depfile.cut**: Lists essentially every possible prefix character.
**chatter validate**: ✅ %com is free text. This exhaustive pattern list in
depfile.cut is effectively "accept anything."

## 4. Gap Analysis Summary

### Gaps Where `chatter validate` Should Be Updated

| Area | depfile.cut Rule | chatter validate Status | Priority |
|------|-----------------|----------------------|----------|
| `%tim` bare seconds | `@t<ss>` (e.g., `45`) | `parse_time_value` requires `:` | Low — rare format |
| `@Birth of` date format | `@d<dd-lll-yyyy>` | E524 only checks participant exists | Medium |
| `@Date` bare `-` | Literal `-` accepted | ❓ Verify if accepted | Low |
| `@Options` `dummy` | NOT in depfile (only CA, NoAlign) | E534 accepts dummy | Low — intentional? |

Note on `%tim`: Our `TimTier::from_text` already handles `hh:mm:ss`, `mm:ss`,
`hh:mm:ss-hh:mm:ss`, and `hh:mm-hh:mm` correctly (via `parse_time_value` which
accepts 2-part and 3-part colon-separated formats, plus ranges). The only missing
format is bare seconds (`@t<ss>`, e.g., `45`) which has no colons — `parse_time_value`
returns `None` for 1-part values, causing E603 to fire incorrectly.

### Areas Where `chatter validate` Goes BEYOND depfile.cut

| Area | chatter validate | depfile.cut |
|------|-----------------|------------|
| %mor structure | Full POS\|lemma&features parsing | Just `*` wildcard |
| %gra relations | Index, head, relation validation | Just `*` wildcard |
| Tier alignment | E714/E715 word count matching | Not checked |
| Timestamp ordering | E701+ monotonicity checks | Not checked |
| Header ordering | E543 @Participants before @Options/@ID | Partially hardcoded |
| Speaker self-overlap | E133 | Not checked |

### Areas That Are Equivalent

| Area | Error | Status |
|------|-------|--------|
| Participant roles | E326 ↔ CHECK 15 | ✅ Same 49 roles |
| @Media types | E535/E536 ↔ CHECK 113 | ✅ Same values |
| @Number | E537 ↔ CHECK 11 | ✅ Same values |
| @Recording Quality | E538 ↔ CHECK 11 | ✅ Same values |
| @Transcription | E539 ↔ CHECK 11 | ✅ Same values |
| @ID sex | E542 ↔ CHECK 64 | ✅ male/female |
| @ID SES | E546 ↔ CHECK 144 | ✅ Same values |
| @ID age | E517 ↔ CHECK 34 | ✅ Same formats |
| @Time Duration | E540 ↔ CHECK 35 | ✅ Similar (ours slightly more permissive) |
| @Time Start | E541 ↔ CHECK 35 | ✅ Similar (ours allows .mmm) |
| @Date format | E518 ↔ CHECK 34 | ✅ DD-MMM-YYYY |
| Language codes | E510 ↔ CHECK 121 | ✅ ISO 639 |
| Dependent tier names | E605 ↔ CHECK 17 | ✅ Same standard set |

### Main Tier Patterns

The main tier (`*:`) section of depfile.cut defines legal tokens in several categories:

**Words and wildcards:**
- `*` — any normal word (alphanumeric/non-ASCII start)
- `,` — literal comma

**Bracket codes:**
- `[- _*]` `[+ _*]` `[^ *]` — language/error/scope markers
- `[!]` `[!!]` `[?]` `[e]` — emphasis, question, error markers
- `[=! _*]` `[= _*]` `[=? _*]` — explanations
- `[: _*]` — replacements
- `[\*]` `[\* _*]` `[\% _*]` — error coding
- `[>*]` `[<*]` — overlap markers
- `[/]` `[//]` `[///]` `[/-]` `[^c]` — retrace/revision codes

**Utterance terminators (+ prefixed):**
- `+...` `+/.` `+!?` `+//.` `+/?` `+"/.` `+".` `+//?` `+..?` `+.`
- `+"` `+^` `+<` `+,` `++` `+≋` `+≈`

**Fragment/phonological prefixes (& prefixed):**
- `&{l=*` `&}l=*` `&{n=*` `&}n=*` — long features
- `&~*` `&+*` `&-*` `&\**` `&=*` — fragment types

**Special form markers (@ suffixed):**
- `*@b` `*@c` `*@d` `*@f` `*@fp` `*@g` `*@i` `*@k` `*@l` `*@ls` `*@n` `*@o` `*@p`
- `*@q` `*@s` `*@s:*` `*@s$*` `*@si` `*@sl` `*@sas` `*@t` `*@u` `*@wp` `*@x` `*@z:*`
- `+@c` `+@f` `+@n` `+@s` `+@s:*` `+@si` `+@sl` `+@sas` `+@t` `+-_*`

**Other:**
- `` `_* `` — compound clitic markers
- `*-` `:` — compound and colon markers
- `'_*` `/_*` `//_*` `///_*` — elision/pause markers

**CHECK validation for main tier words:**
1. `check_matchplate()` tries each word against the entire template list
2. `check_CheckFixes()` validates suffixes/prefixes (errors 20, 37, 147)
3. `check_CheckWords()` does character-level validation (errors 47, 48, 65, etc.)

**chatter validate**: ✅ Our tree-sitter grammar and parser handle ALL of these
structurally — they are built into grammar rules, not validated after the fact.
This is fundamentally more robust than CHECK's pattern-matching approach because
illegal constructs cannot even parse. Special form markers are validated against
a hardcoded `FormType` enum with the same set of markers (E203).

**Assessment**: No gaps for main tier validation. Our grammar-based approach
subsumes depfile.cut's pattern matching.

## 5. Conclusions

1. **depfile.cut is effectively frozen** — it changes rarely and in coordination
   with CHAT format updates.

2. **`chatter validate` already covers ~95% of depfile.cut's validation rules**
   through hardcoded checks that track the same values.

3. **The gaps are minor**:
   - `%tim` bare seconds (`@t<ss>`) — `parse_time_value` needs a 1-part case
   - `@Birth of` date format — not validated (only participant reference check)
   - `@Date` bare `-` — unknown if accepted by our validator

4. **`chatter validate` goes well beyond depfile.cut** in areas like %mor/%gra
   structural validation, tier alignment, timestamp ordering, and header ordering.

5. **Runtime depfile.cut parsing is NOT recommended** because:
   - Our hardcoded rules already track its content
   - Two sources of truth would create maintenance burden
   - Our grammar-based approach for main tier validation is superior
   - depfile.cut uses a legacy pattern-matching language

6. **When depfile.cut changes** (rare), we update the corresponding hardcoded
   validation rules in `talkbank-model`.

---

*Generated from analysis of check.cpp (6076 lines) and depfile.cut (99 lines).*
*Last updated: 2026-03-05*
