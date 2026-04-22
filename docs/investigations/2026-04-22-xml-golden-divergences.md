# XML Golden Parity Divergences — Assessment

**Status:** Draft — awaiting per-pattern rulings before code changes.
**Last updated:** 2026-04-22 06:52 EDT
**Scope:** After the depfile.cut conformance sweep (commits `5dbee4c6`,
`a3d4eaa9`) and Franklin's regeneration of `corpus/reference-xml/` from
updated Java Chatter, 60/83 golden-parity tests pass. This doc
catalogues the 9 distinct divergence patterns behind the remaining 23
failures so each can be ruled Rust-bug / Java-bug / spec-ambiguous.

Column meanings:
- **Source CHAT** — the exact input that produces the divergence.
- **Rust emits** — current `talkbank-transform` XML output.
- **Java emits** — `reference-xml/*.xml` golden content.
- **My read** — who I think is right, with a one-line reason.
- **Verdict needed?** — ruling required from Franklin, or obvious?

---

## Pattern 1 — Compound word with replacement: text split

**Files affected:** `annotation/errors-and-replacements.cha` (first
divergence; may cascade to others).

**Source CHAT:**
```
*MOT:	rocking+house [: rocking+horse] [*] ?
```

**Rust emits:** `<w>rockinghouse</w>` (whole compound as one text node)

**Java emits:** `<w>rocking</w>...<w>horse</w>` (split into components,
with structured `<mw>` compound wrapper around the parts)

**My read:** **Java right.** The `+` in `word+word` is a compound-word
marker with its own XML element model (`<mw><mpfx>...</mpfx><stem>...
</stem></mw>`); emitting the whole compound as a flat text blob drops
the structural information the XSD encodes. Rust's emitter is missing
the compound decomposition.

**Verdict needed?** No — Rust bug, fix it.

---

## Pattern 2 — Bullet timestamp leaks into text content

**Files affected:** `content/media-bullets.cha`,
`core/headers-comments.cha` (at minimum — may cascade).

**Source CHAT:**
```
@Comment:	CHI points to the shelf ·1234_1567·
```
(where `·` is the bullet delimiter U+0015)

**Rust emits:** `<comment>CHI points to the shelf [1234_1567]</comment>`
(bullet converted to `[n_n]` and kept in the text)

**Java emits:** `<comment>CHI points to the shelf</comment>`
(bullet extracted, emitted as a separate `<media>` element next to the
comment; text is clean)

**My read:** **Java right.** Timing bullets in `@Comment` / `@Gem`
content should project to structural `<media>` elements, not stay
embedded as `[start_end]` text. My earlier XML emission work (commit
`63902849`) chose `[start_end]` text as a workaround for the U+0015
character being rejected by XML 1.0; that workaround was meant to be
temporary but shipped. The real fix is to extract the bullet into a
sibling `<media>` (or skip the bullet entirely in comment scope, per
the Java behavior).

**Verdict needed?** No — Rust bug, fix it. Likely reuses the structural
bullet-extraction logic already in `xml/root.rs` but needs to apply to
`@Comment` / `@Gem` / `@Situation` bullet-content paths.

---

## Pattern 3 — Pause length: trailing-zero formatting

**Files affected:** `content/pauses-and-events.cha`,
`content/pauses-timed.cha`.

**Source CHAT:** `(3.4)` or `(0.5)` — a timed pause in parens.

**Rust emits:** `<pause length="3.400" symbolic-length="simple"/>`

**Java emits:** `<pause length="3.4" symbolic-length="simple"/>`

**My read:** **Java right.** Java preserves the author's decimal
precision; Rust expands `3.4` to `3.400` via three-decimal
zero-padding. The XSD's `duration` type is `xs:decimal` which accepts
both, but the canonical form is un-padded. This is a formatting-only
divergence with no semantic difference.

**Verdict needed?** No — Rust emitter bug; fix to emit the number
without trailing-zero padding.

---

## Pattern 4 — `<CHAT>` attributes: legacy CLAN GUI state

**Files affected:** `content/pauses-and-events.cha`,
`core/headers-pre-begin.cha`, possibly others.

**Source CHAT:** File has no explicit GUI-state header (no `@Window:`
header etc. — these are implicit).

**Rust emits:**
```xml
<CHAT Corpus="corpus" Lang="eng" Version="3.2.3" ...>
```

**Java emits:**
```xml
<CHAT Colorwords="*CHI 1 32768 0 656 *MOT 1 656 33423 1311"
      Corpus="corpus" Font="Win:Courier New:14:25" Lang="eng" PID="..."
      Version="3.2.3" Window="247_181_683_700_-1_-1_348_0_348_0" ...>
```

**My read:** **Java wrong.** `Colorwords`, `Font`, `Window` are
vestigial CLAN GUI state — historically CLAN editor stored window
position, color mappings per speaker, and editor font inside the
CHAT/XML. That metadata has no semantic content; it's UI state that
should not propagate through a pure XML conversion. The fact that
Java emits them means it's reading them from somewhere (maybe the
`.cha` has them, maybe defaults), but the canonical CHAT manual does
not define these as XML attributes.

**Verdict needed?** **Yes — please rule.** If you agree they're
legacy GUI leak-through, we don't make Rust emit them. If they're
genuinely needed for some CLAN GUI round-trip, we add them. I lean
"don't add."

---

## Pattern 5 — Per-utterance `xml:lang` projection

**Files affected:** `content/language-switching.cha`,
`edge-cases/multi-language.cha`.

**Source CHAT:**
```
@Languages:	eng, spa
*MOT:	hola [- spa] niño .
```

**Rust emits:** `<u uID="u1" who="MOT">...</u>`

**Java emits:** `<u uID="u1" who="MOT" xml:lang="spa">...</u>`

**My read:** **Java right.** When a `[- LANG]` pre-code tells the
reader that this utterance is in a specific language (different from
the file's default), Java projects that onto the `<u>` element as
`xml:lang`. Rust currently doesn't. The pre-code → `xml:lang` mapping
is a semantic projection that the Rust XML emitter is missing.

**Verdict needed?** Probably no — Rust bug. But there's a design
decision: does the `xml:lang` only fire when the pre-code EXPLICITLY
names a language, or does it also fire when the file has multiple
`@Languages` and an `[- lang]` code? I'll assume the former (strict
"only when explicit pre-code exists").

---

## Pattern 6 — `Mediatypes` attribute: should status be appended?

**Files affected:** `core/headers-media.cha`,
`core/headers-media-url.cha`.

**Source CHAT:**
```
@Media:	sample, audio, missing
```

**Rust emits:** `<CHAT ... Mediatypes="audio" Media="...">`

**Java emits:** `<CHAT ... Mediatypes="audio missing" Media="...">`

**My read:** **Java right per XSD, but worth a sanity check.** Looking
at the XSD's `Mediatypes` definition: it's a space-separated list
type whose enumeration includes `audio | video | unlinked | missing |
notrans`. Java appends the status (`missing` / `unlinked` / `notrans`)
when present; Rust emits only the type. This is arguably cleaner
(the status is already in the separate data model) but the XSD-
sanctioned shape is the Java one.

**Verdict needed?** **Yes please.** Minor — Java's shape is in the
XSD so if we care about XSD conformance (we do, that's our
`xml_schema_validate` gate), Rust should append.

---

## Pattern 7 — `<media skip="true"/>` attribute

**Files affected:** `content/pauses-timed.cha` (possibly only file).

**Source CHAT:** Presumably `·-start_end·` — a bullet marked with a
leading hyphen in CHAT's bullet notation.

**Rust emits:** `<media start="357.000" end="357.477" unit="s"/>`

**Java emits:** `<media start="357.000" end="357.477" skip="true"
unit="s"/>`

**My read:** **Java right.** The `-` prefix on a bullet means
"this segment is a skip marker" (non-speech region to exclude from
analysis). Java captures it via `skip="true"`; Rust emits the time
range but loses the skip flag. Rust model needs the skip-flag
handling.

**Verdict needed?** No — Rust bug. Worth confirming the exact
source-CHAT notation so the fix isn't speculative.

---

## Pattern 8 — Retrace kind attribute spelling

**Files affected:** `annotation/retrace.cha`.

**Source CHAT:**
```
*CHI:	I went [//] went home .
*CHI:	the cake was <so big> [///] huge .
```
(`[//]` = retrace with correction; `[///]` = reformulation)

**Rust emits:** `<k type="retracing with correction"/>`

**Java emits:** `<k type="retracing reformulation"/>`

**My read:** **Needs inspection per case.** Rust's "retracing with
correction" is XSD-valid (it's an enumeration value). Java's
"retracing reformulation" is also XSD-valid. These are different
semantic categories:

| CHAT notation | Meaning | XSD enum value |
|---|---|---|
| `[/]` | Repetition | `retracing` |
| `[//]` | Retrace with correction | `retracing with correction` |
| `[///]` | Reformulation | `retracing reformulation` |
| `[/-]` | False start | `false start` |
| `[/?]` | Uncertain retrace | `uncertain retracing` |

So if the source was `[///]` (reformulation), Java is right and Rust
is wrong. If source was `[//]`, Rust is right and Java is wrong.
Need to look at the specific utterance.

**Verdict needed?** **Yes — I need to look at the specific failing
file to say.** Will do as part of the fix.

---

## Pattern 9 — `<postcode/>` element vs text: one-vs-many utterances

**Files affected:** `content/postcodes-and-freecodes.cha`,
`edge-cases/postcodes-and-gems.cha`.

**Source CHAT:**
```
*CHI:	hello . [+ trn]
```
(`[+ trn]` is a postcode)

**Rust emits:** `<u>...<t type="p"/></u>` (postcode dropped entirely)

**Java emits:** `<u>...<t type="p"/><postcode>trn</postcode></u>`

**My read:** **Java right.** `[+ code]` is a postcode that should
project to a `<postcode>` element; Rust's emitter is dropping it.
Postcodes are documented in the CHAT manual and have their own XSD
element.

**Verdict needed?** No — Rust bug, fix it.

---

## Pattern 10 — `<mor>` stem element with trailing affix text

**Files affected:** `tiers/phon-syllabification.cha`.

**Source CHAT (%mor tier):**
```
%mor:	v|go-PAST&3S pro|I .
```

**Rust emits:**
```xml
<mor>
  <mw><pos><c>v</c></pos><stem>gPRES</stem></mw>
  ...
```
(Note: stem contains "gPRES" — looks mangled)

**Java emits:**
```xml
<mor>
  <mw><pos><c>v</c></pos><stem>go</stem><mk type="sfx">PAST</mk>...</mw>
  ...
```

**My read:** **Rust has a real bug in %mor affix parsing.** The stem
should be `go`, with `-PAST` as a separate `<mk type="sfx">PAST</mk>`
element. Rust's emitter seems to be gluing the affix label into the
stem text, producing `gPRES` (truncated? concatenated?). This is
beyond a simple divergence — Rust's %mor emission for affixed words
is broken.

**Verdict needed?** No — Rust bug, but it's a non-trivial fix. May
want to carve this out as its own spec+test.

---

## Pattern 11 — Double-space preservation in comments

**Files affected:** `edge-cases/special-terminators.cha`.

**Source CHAT:**
```
@Comment:	... (+".),  quoted new line ...
```
(Two spaces after `(+".),`)

**Rust emits:** preserves the two spaces.

**Java emits:** collapses to one space.

**My read:** **Java right, but trivially.** Whitespace collapse in
text elements is XML-spec-acceptable either way; canonical XML
collapses. Rust's fidelity preservation is debatable.

**Verdict needed?** Probably no — match Java, collapse runs of ASCII
whitespace in comment text.

---

## Pattern 12 — Multi-line comment with tab continuation

**Files affected:** `core/headers-pre-begin.cha`.

**Source CHAT:**
```
@Situation:	Free play session in the lab playroom with a variety of toys
	available on the floor and table
```

**Rust emits:** `<comment type="Situation">Free play session... toys\n\tavailable on the floor and table</comment>`

**Java emits:** `<comment type="Situation">Free play session... toys available on the floor and table</comment>` (newline+tab collapsed to a single space)

**My read:** **Java right.** CHAT's continuation-line convention
(tab-indented continuation of a header) is a source-formatting
artifact, not a meaningful newline. Java collapses; Rust preserves.
Same class as Pattern 11 — whitespace normalization in header text.

**Verdict needed?** No — Rust bug, match Java.

---

## Summary of rulings needed

Two patterns where I'd like an explicit ruling before I fix:

- **#4 Legacy CLAN GUI attributes (`Window`, `Colorwords`, `Font`)** — My read is "Java wrong, leave them out of Rust." Confirm?
- **#6 Mediatypes-status concatenation** — My read is "Java right (XSD-sanctioned), Rust should append." Confirm?

Everything else is a Rust emitter bug with clear remediation. If you
rule on #4 and #6 I can plan the fix sequence.

## Suggested fix order

Ordered by surface area (small fixes first, then the big one):

1. **Pause length trailing-zero strip** (Pattern 3) — 1-line fix in
   `xml/root.rs` or wherever pause `length` is formatted.
2. **Whitespace collapse in header/comment text** (Patterns 11, 12) —
   Apply `str::split_whitespace().collect::<Vec<_>>().join(" ")` to
   text before emission.
3. **`Mediatypes` status append** (Pattern 6, pending ruling) — 1-line
   shape change in `<CHAT>` attribute emission.
4. **`<postcode/>` element emission** (Pattern 9) — add the element
   type to the XML emitter's utterance-item walker.
5. **`<media skip="true"/>` flag** (Pattern 7) — plumb the skip flag
   from the model through to `<media>` emission.
6. **Bullet → `<media>` for comment-content** (Pattern 2) — move the
   `[start_end]` text-path to a structural extraction.
7. **Per-utterance `xml:lang` from `[- LANG]` pre-code** (Pattern 5) —
   project pre-code language into `<u>` attribute.
8. **Retrace-kind attribute correctness** (Pattern 8) — map the CHAT
   notation (`[//]` vs `[///]` vs `[/-]` etc.) to the XSD enumeration
   value correctly.
9. **Compound word decomposition** (Pattern 1) — generate `<mw>`
   structure with decomposed parts.
10. **%mor affix emission** (Pattern 10) — fix the stem/affix split,
    emit `<mk>` elements for each affix.

1–3 are probably an hour together. 4–7 are maybe a half-day. 8–10
are each a full-day's careful spec-and-test work.
