# E101: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata
- **Status**: not_implemented

- **Error Code**: E101
- **Category**: validation
- **Level**: file
- **Layer**: parser

## Example 1

**Source**: `error_corpus/parse_errors/E101_invalid_line_format.cha`
**Trigger**: See example below
**Expected Error Codes**: E101

```chat
@Begin
@Languages:	eng
InvalidLine
@Comment:	ERROR: Line format invalid
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: See example above

## CHAT Rule

See the CHAT manual for format specifications: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
