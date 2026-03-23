# E701: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata
- **Status**: not_implemented
- **Layer**: validation

- **Error Code**: E701
- **Category**: Dependent tier parsing
- **Level**: tier
- **Layer**: validation

## Example 1

**Source**: `E7xx_tier_parsing/E701_empty_mor_chunk.cha`
**Trigger**: %mor tier with empty chunk (consecutive spaces)
**Expected Error Codes**: E701

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hello world .
%mor:	v|hello  n|world .
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
