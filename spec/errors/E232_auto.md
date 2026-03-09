# E232: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: E232
- **Category**: validation
- **Level**: word
- **Layer**: validation

## Example 1

**Source**: `error_corpus/validation_errors/E232_compound_marker_at_start.cha`
**Trigger**: See example below
**Expected Error Codes**: E232

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
@Comment:	ERROR: Compound marker (+) cannot be at word start
@Comment:	Invalid: '+hello' - Compound marker at start
*CHI:	+hello .
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
