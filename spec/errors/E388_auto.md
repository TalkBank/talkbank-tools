# E388: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata
- **Status**: not_implemented

- **Error Code**: E388
- **Category**: validation
- **Level**: utterance
- **Layer**: validation

## Example 1

**Source**: `E2xx_word_errors/E388_replacement_on_nonword.cha`
**Trigger**: See example below
**Expected Error Codes**: E311, E316

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	&=laugh [: well] .
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
