# E723: GRA has multiple ROOTs

## Description

GRA has multiple ROOTs

## Metadata

- **Error Code**: E723
- **Category**: validation
- **Level**: tier
- **Layer**: parser
- **Status**: not_implemented

## Example 1

**Source**: `error_corpus/validation_errors/E723_gra_multiple_roots.cha`
**Trigger**: Multiple relations with head=0 (ROOT)
**Expected Error Codes**: E723

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
*CHI:	I want .
%mor:	pro|I v|want .
%gra:	1|0|ROOT 2|0|ROOT .
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
