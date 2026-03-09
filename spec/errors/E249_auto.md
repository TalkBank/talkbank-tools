# E249: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: E249
- **Category**: validation
- **Level**: word
- **Layer**: validation

## Example 1

**Source**: `error_corpus/validation_errors/E249_missing_language_context.cha`
**Trigger**: See example below
**Expected Error Codes**: E249

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
@Comment:	ERROR: Language marker without context
@Comment:	Invalid: Language marker usage without declaration
*CHI:	hello@s .
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
