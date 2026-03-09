# E700: Unexpected tier node

## Description

Unexpected tier node

## Metadata

- **Error Code**: E700
- **Category**: validation
- **Level**: tier
- **Layer**: validation
- **Status**: not_implemented

## Example 1

**Source**: `error_corpus/validation_errors/E700_unexpected_tier_node.cha`
**Trigger**: Tier body contains unexpected node type
**Expected Error Codes**: E700

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
*CHI:	hello .
%mor:	pro|I v|want .
@End
```

## Expected Behavior

The parser should successfully parse this CHAT file, but validation should report the error.

**Trigger**: See example above

## CHAT Rule

See CHAT manual sections on dependent tier formats (%mor, %gra, %pho, etc.). Each tier type has specific syntax requirements. The CHAT manual is available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
