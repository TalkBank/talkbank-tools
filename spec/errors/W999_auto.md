# W999: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: W999
- **Category**: validation
- **Level**: file
- **Layer**: validation

## Example 1

**Source**: `W_warnings/W999_legacy_warning.cha`
**Trigger**: See example below
**Expected Error Codes**: W999

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
@OldHeader:	some value
*CHI:	hello .
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
