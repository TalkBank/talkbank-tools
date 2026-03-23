# E248: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata
- **Status**: not_implemented
- **Layer**: validation

- **Error Code**: E248
- **Category**: validation
- **Level**: word
- **Layer**: validation

## Example 1

**Source**: `error_corpus/validation_errors/E248_tertiary_without_code.cha`
**Trigger**: See example below
**Expected Error Codes**: E248

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
@Languages:	eng, spa
@Comment:	ERROR: Tertiary language (@t) requires explicit code
@Comment:	Invalid: 'hello@t' - Must specify language code
*CHI:	hello@t .
@End
```

## Expected Behavior

The parser should successfully parse this CHAT file, but validation should report the error.

**Trigger**: See example above

## CHAT Rule

See CHAT manual sections on word-level syntax and special markers. The CHAT manual is available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
