# E212: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: E212
- **Category**: Parser error
- **Level**: word
- **Layer**: validation
- **Status**: implemented

## Example 1

**Source**: `E2xx_word_errors/E212_unexpected_text.cha`
**Trigger**: Malformed word syntax caught by parser
**Expected Error Codes**: E212

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
@Comment:	Note: This is a parser error, hard to trigger with valid grammar
*CHI:	hello world .
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
