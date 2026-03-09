# E213: Replacement word cannot be untranscribed

## Description

Replacement word cannot be untranscribed

## Metadata

- **Error Code**: E213
- **Category**: Word validation
- **Level**: word
- **Layer**: validation

## Example 1

**Source**: `E2xx_word_errors/E213_unexpected_scoped_annotation.cha`
**Trigger**: Replacement containing xxx (untranscribed marker)
**Expected Error Codes**: E391

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	helo [: xxx] world .
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
