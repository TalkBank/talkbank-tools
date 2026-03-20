# E251: Empty word content text

## Description

Empty word content text

## Metadata

- **Error Code**: E251
- **Category**: validation
- **Level**: word
- **Layer**: parser
- **Status**: implemented

## Example 1

**Source**: `error_corpus/validation_errors/E251_empty_word_content_text.cha`
**Trigger**: Word with annotations but empty text
**Expected Error Codes**: E251

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
*CHI:	@s:eng .
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
