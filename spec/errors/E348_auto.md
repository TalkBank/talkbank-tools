# E348: Unpaired overlap marker within utterance

## Description

An opening overlap marker (⌈ or ⌊) has no matching closing marker (⌉ or ⌋)
within the same utterance, or a closing marker appears without a preceding
opening marker. Reported as a warning because onset-only marking (⌈ without ⌉)
is a legitimate CA convention.

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
