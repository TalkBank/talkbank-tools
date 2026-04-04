# E360: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Status**: not_implemented
- **Last updated**: 2026-04-04 08:15 EDT
- **Error Code**: E360
- **Category**: validation
- **Level**: utterance
- **Layer**: parser

## Example 1

**Source**: `E3xx_main_tier_errors/E360_invalid_media_bullet.cha`
**Trigger**: Zero-duration timestamp (start == end, both 0ms) — triggers E362 instead
**Expected Error Codes**: E362

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hello . 0_0
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
