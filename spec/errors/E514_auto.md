# E514: Missing language code in @ID

## Description

Missing language code in @ID

## Metadata

- **Error Code**: E514
- **Category**: validation
- **Level**: header
- **Layer**: validation
- **Status**: not_implemented

## Example 1

**Source**: `error_corpus/validation_errors/E514_missing_language_code.cha`
**Trigger**: See example below
**Expected Error Codes**: E514

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	|corpus|CHI|||||Child|||
*CHI:	hello .
@End
```

## Expected Behavior

The parser should successfully parse this CHAT file, but validation should report the error.

**Trigger**: See example above

## CHAT Rule

See CHAT manual sections on file headers and metadata. Headers like @Participants, @Languages, and @ID have specific format requirements. The CHAT manual is available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
