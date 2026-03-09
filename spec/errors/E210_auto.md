# E210: Replacement not allowed for phonological fragment

## Description

Replacement not allowed for phonological fragment

## Metadata

- **Error Code**: E210
- **Category**: Word validation
- **Level**: word
- **Layer**: validation

## Example 1

**Source**: `E2xx_word_errors/E210_replacement_missing_arrow.cha`
**Trigger**: Replacement on word with &+ prefix
**Expected Error Codes**: E387

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	&+fri [: friend] world .
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
