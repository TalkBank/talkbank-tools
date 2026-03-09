# W724: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: W724
- **Category**: validation
- **Level**: tier
- **Layer**: parser
- **Status**: not_implemented

## Example 1

**Source**: `error_corpus/warnings/W724_gra_root_head_not_self.cha`
**Trigger**: ROOT relation where head index does not point to self
**Expected Error Codes**: W724

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
*CHI:	I want .
%mor:	pro|I v|want .
%gra:	1|2|SUBJ 2|1|ROOT .
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: See example above

## CHAT Rule

See the CHAT manual for format specifications: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
