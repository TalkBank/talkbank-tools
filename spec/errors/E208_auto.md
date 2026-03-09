# E208: Empty replacement

## Description

Empty replacement

## Metadata

- **Error Code**: E208
- **Category**: validation
- **Level**: word
- **Layer**: parser
- **Status**: not_implemented

## Example 1

**Source**: `error_corpus/validation_errors/E208_empty_replacement.cha`
**Trigger**: Replacement with empty target
**Expected Error Codes**: E208

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
*CHI:	hello [: ] .
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: See example above

## CHAT Rule

See CHAT manual sections on word-level syntax and special markers. The CHAT manual is available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
