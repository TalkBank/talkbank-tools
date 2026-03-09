# E245: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: E245
- **Category**: validation
- **Level**: word
- **Layer**: validation
- **Status**: not_implemented

## Example 1

**Source**: `error_corpus/validation_errors/E245_stress_without_material.cha`
**Trigger**: See example below
**Expected Error Codes**: E245

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
@Comment:	ERROR: Stress marker must precede spoken material
@Comment:	Invalid: 'ˈ' - Stress without following text
*CHI:	ˈ .
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
