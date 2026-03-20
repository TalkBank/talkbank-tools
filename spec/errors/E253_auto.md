# E253: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: E253
- **Category**: validation
- **Level**: word
- **Layer**: validation
- **Status**: implemented

## Example 1

**Source**: `error_corpus/validation_errors/E253_empty_word_content.cha`
**Trigger**: See example below
**Expected Error Codes**: E253

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
@Comment:	ERROR: Word must have content
@Comment:	Invalid: Empty word element
*CHI:	  .
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
