# E721: GRA non-sequential index

## Description

GRA non-sequential index

## Metadata

- **Error Code**: E721
- **Category**: validation
- **Level**: tier
- **Layer**: parser
- **Status**: not_implemented

## Example 1

**Source**: `error_corpus/validation_errors/E721_gra_non_sequential.cha`
**Trigger**: GRA indices not in sequential order
**Expected Error Codes**: E721

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
*CHI:	I want cookie .
%mor:	pro|I v|want n|cookie .
%gra:	1|2|SUBJ 3|2|OBJ 2|0|ROOT .
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: See example above

## CHAT Rule

See CHAT manual sections on dependent tier formats (%mor, %gra, %pho, etc.). Each tier type has specific syntax requirements. The CHAT manual is available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
