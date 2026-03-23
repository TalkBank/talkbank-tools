# E347: Unbalanced cross-speaker overlap

## Description

An **indexed** top overlap region (⌈2...⌉2) on one speaker has no matching
indexed bottom overlap region (⌊2...⌋2) from a different speaker within the
nearby utterances, or vice versa. Reported as a warning.

**Unindexed markers are suppressed.** Multi-party overlaps without numeric indices
are inherently ambiguous — the machine matcher cannot determine which unindexed
top corresponds to which unindexed bottom when multiple speakers overlap
simultaneously. See `docs/overlap-validation-audit.md` in talkbank-dev for the
full investigation (SBCSAE, CLAPI, Forrester examples).

## Metadata
- **Status**: not_implemented
- **Layer**: validation

- **Error Code**: E347
- **Category**: validation
- **Level**: cross_utterance
- **Layer**: validation

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
