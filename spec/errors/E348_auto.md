# E348: Unpaired overlap marker within utterance

## Description

A closing overlap marker (⌉ or ⌋) appears without a preceding opening marker
(⌈ or ⌊) within the same utterance. Reported as a warning.

**Onset-only marking (opening without closing) is suppressed.** An opening marker
without a matching close is standard Jeffersonian CA convention — the transcriber
marks where overlap begins, with the end implied by the end of the shorter turn.
See `docs/overlap-validation-audit.md` in talkbank-dev for the full investigation.

Markers are matched by kind (top/bottom) and index (2-9 or unindexed).

## Metadata

- **Error Code**: E348
- **Category**: validation
- **Level**: utterance
- **Layer**: validation
- **Status**: implemented

## Example 1

**Source**: `error_corpus/parse_errors/E348_missing_overlap_end.cha`
**Trigger**: Overlap begin marker without matching end
**Expected Error Codes**: E348

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child, MOT Mother
@ID:	eng|corpus|CHI|||||Target_Child|||
@ID:	eng|corpus|MOT|||||Mother|||
*CHI:	hello ⌈ world .
*MOT:	yes .
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: See example above

## CHAT Rule

See CHAT manual sections on main tier syntax and utterance structure. Every utterance must end with a terminator (., ?, or \!). The CHAT manual is available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
