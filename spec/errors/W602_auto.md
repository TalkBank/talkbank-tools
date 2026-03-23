# W602: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata
- **Status**: not_implemented
- **Layer**: validation

- **Error Code**: W602
- **Category**: Warnings
- **Level**: tier
- **Layer**: validation

## Example 1

**Source**: `W6xx_warnings/W602_deprecated_xtier.cha`
**Trigger**: %xpho should be updated to %pho
**Expected Error Codes**: W602

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hello world .
%xpho:	hɛloʊ wɜɹld
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
