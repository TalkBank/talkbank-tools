# E365: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: E365
- **Category**: validation
- **Level**: utterance
- **Layer**: validation
- **Status**: implemented

## Example 1

**Source**: `E3xx_main_tier_errors/E365_malformed_tier_content.cha`
**Trigger**: See example below
**Expected Error Codes**: E365

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hello .
%pho:	***bad***
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
