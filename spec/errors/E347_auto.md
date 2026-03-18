# E347: Unbalanced cross-speaker overlap

## Description

A top overlap region (⌈...⌉) on one speaker has no matching bottom overlap
region (⌊...⌋) from a different speaker within the nearby utterances, or vice
versa. Reported as a warning because onset-only marking is a legitimate CA
convention in some corpora.

## Metadata

- **Error Code**: E347
- **Category**: validation
- **Level**: cross_utterance
- **Layer**: validation
- **Status**: implemented

## Example 1

**Source**: `E3xx_main_tier_errors/E347_unbalanced_overlap.cha`
**Trigger**: See example below
**Expected Error Codes**: E347

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child, MOT Mother
@ID:	eng|corpus|CHI|||||Child|||
@ID:	eng|corpus|MOT|||||Mother|||
*CHI:	⌈hello⌉ .
*MOT:	hi .
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
