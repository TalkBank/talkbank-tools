# E344: Invalid scoped annotation nesting

## Description

Invalid scoped annotation nesting

## Metadata
- **Status**: not_implemented

- **Error Code**: E344
- **Category**: validation
- **Level**: utterance
- **Layer**: parser

## Example 1

**Source**: `error_corpus/parse_errors/E344_invalid_scoped_nesting.cha`
**Trigger**: Overlapping scoped annotations of same type
**Expected Error Codes**: E344

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
*CHI:	hello <world <foo> bar> .
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: See example above

## CHAT Rule

See CHAT manual sections on main tier syntax and utterance structure. Every utterance must end with a terminator (., ?, or \!). The CHAT manual is available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
