# E209: Empty spoken content

## Description

Empty spoken content

## Metadata

- **Error Code**: E209
- **Category**: validation
- **Level**: word
- **Layer**: parser
- **Status**: not_implemented

## Example 1

**Source**: `error_corpus/validation_errors/E209_empty_spoken_content.cha`
**Trigger**: Word with form marker but no spoken text
**Expected Error Codes**: E209

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
*CHI:	@l .
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
