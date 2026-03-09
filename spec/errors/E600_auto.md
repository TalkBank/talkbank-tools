# E600: Tier alignment skipped due to parse errors

## Description

A dependent tier (typically `%mor`) had parse errors during lenient recovery, so the
validator cannot verify alignment between tiers. Alignment checks (main↔%mor, %mor↔%gra)
are skipped for the affected utterance. This is a **warning**, not an error — the file
still parses, but alignment correctness is unverified for tainted tiers.

E600 fires in pairs: if `%mor` is tainted, both main↔%mor and %mor↔%gra alignment
checks are skipped, producing two E600 warnings for the same utterance.

## Metadata

- **Error Code**: E600
- **Category**: validation
- **Level**: tier
- **Layer**: validation
- **Status**: not_implemented

## Example 1 — CLAN scope marker in %mor

**Source**: `ca-data/MICASE/lel/lel542su096.cha` (line 97)
**Trigger**: `&}l=READING` in `%mor` — a CLAN postcoding scope marker injected into
the morphosyntax tier. Not valid %mor syntax.

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	S1 Teacher
@ID:	eng|corpus|S1|||||Teacher|||
*S1:	to be Jewish was enough to escape (.) the anonymity of an identity indistinguishable from others (.) and the dullness of an &~unevent uneventful life . 226898_236744
%mor:	part|to aux|be-Inf-S adj|Jewish-S1 aux|be-Fin-Ind-Past-S3-irr adj|enough-S1 part|to verb|escape-Inf-S det|the-Def-Art noun|anonymity-Acc adp|of det|a-Ind-Art noun|identity adj|indistinguishable-S1 adp|from noun|other-Plur cconj|and det|the-Def-Art noun|dullness adp|of det|a-Ind-Art adj|uneventful-S1 noun|life &}l=READING .
%gra:	1|3|MARK 2|3|COP 3|5|CSUBJ 4|5|COP 5|22|ROOT 6|7|MARK 7|5|CSUBJ 8|9|DET 9|7|OBJ 10|12|CASE 11|12|DET 12|9|NMOD 13|12|AMOD 14|15|CASE 15|13|OBL 16|18|CC 17|18|DET 18|12|CONJ 19|22|CASE 20|22|DET 21|22|AMOD 22|18|NMOD 23|5|PUNCT
@End
```

**Expected**: Two E600 warnings — main↔%mor alignment skipped AND %mor↔%gra alignment
skipped, both because `%mor` is tainted by the `&}l=READING` parse error.

## Example 2 — Missing stem in %mor word

**Source**: `childes-data/Eng-NA/MacWhinney/070518a.cha` (line 1800)
**Trigger**: `noun|-Acc` in `%mor` — the stem is missing (empty string before the pipe
and `-Acc` suffix). This also triggers E342 (missing stem) and E331 (unexpected node).

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	MAR Target_Child
@ID:	eng|corpus|MAR|||||Target_Child|||
*MAR:	don't give me a YM . 1520652_1522904
%mor:	aux|do-Fin-Imp-S~part|not verb|give-Fin-Imp-S pron|I-Prs-Acc-S1 det|a-Ind-Art noun|-Acc .
%gra:	1|3|AUX 2|3|ADVMOD 3|6|ROOT 4|3|IOBJ 5|6|DET 6|3|OBJ 7|3|PUNCT
@End
```

**Expected**: Two E600 warnings — main↔%mor and %mor↔%gra alignment both skipped
because `%mor` is tainted. Also triggers E342 and E331 on the same `noun|-Acc` token.

## Corpus Impact

- 2 files, 4 total E600 warnings (2 per file: main↔%mor + %mor↔%gra)
- `ca-data/MICASE/lel/lel542su096.cha` — CLAN `&}l=READING` scope marker
- `childes-data/Eng-NA/MacWhinney/070518a.cha` — stemless `noun|-Acc`
- Very rare; most %mor tiers parse clean

## Root Cause

E600 is a downstream consequence, not a primary error. The actual problem is in the
%mor tier content (scope marker, missing stem, etc.). When lenient parsing encounters
a parse error in %mor, it marks the tier as "tainted" and continues. The alignment
validator then skips alignment checks for tainted tiers and reports E600 as a warning.

## CHAT Rule

Dependent tiers must parse cleanly for alignment validation to run. See CHAT manual
sections on %mor tier format: each word must be `pos|stem(-suffix)*`.

## Notes

- E600 always fires in pairs (main↔%mor and %mor↔%gra) because both alignments
  depend on a clean %mor parse
- Fix the underlying %mor parse error (the primary error code) and E600 goes away
- Re-running morphotag will regenerate %mor from scratch, eliminating the bad content
