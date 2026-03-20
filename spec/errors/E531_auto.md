# E531: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: E531
- **Category**: validation
- **Level**: header
- **Layer**: validation
- **Status**: implemented

## Example 1

**Source**: `error_corpus/validation_errors/E531_media_filename_mismatch.cha`
**Trigger**: See example below
**Expected Error Codes**: E531

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
@Media:	different, audio
@Comment:	ERROR: Media filename must match transcript
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
