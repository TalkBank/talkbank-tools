# W603: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: W603
- **Category**: Warnings
- **Level**: tier
- **Layer**: validation
- **Status**: not_implemented

## Example 1

**Source**: `W6xx_warnings/W603_empty_header_content.cha`
**Trigger**: Non-required header with no content
**Expected Error Codes**: W603

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
@Comment:	Note: @Comment is optional, so empty is warning not error
@Comment:	
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
