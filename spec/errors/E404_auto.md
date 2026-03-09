# E404: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: E404
- **Category**: validation
- **Level**: tier
- **Layer**: validation

## Example 1

**Source**: `error_corpus/validation_errors/E404_orphaned_dependent_tier.cha`
**Trigger**: See example below
**Expected Error Codes**: E404

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
@Comment:	ERROR: Dependent tier without preceding main tier
@Comment:	Invalid: %mor without *CHI:
%mor:	pro|I v|want n|cookie .
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: See example above

## CHAT Rule

See CHAT manual sections on dependent tier syntax (%mor, %gra, etc.). The CHAT manual is available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
