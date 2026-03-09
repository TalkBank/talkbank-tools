# E383: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: E383
- **Category**: Dependent tier parsing
- **Level**: utterance
- **Layer**: parser
- **Status**: not_implemented

## Example 1

**Source**: `error_corpus/E7xx_tier_parsing/E709_gra_missing_index.cha`
**Trigger**: %gra relation with empty index field
**Expected Error Codes**: E708

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hello world .
%gra:	|2|SUBJ 2|0|ROOT
@End
```

## Example 2

**Source**: `error_corpus/E7xx_tier_parsing/E708_invalid_gra_format.cha`
**Trigger**: %gra relation without enough pipe separators
**Expected Error Codes**: E710

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hello world .
%gra:	1-2-SUBJ 2|0|ROOT
@End
```

## Example 3

**Source**: `error_corpus/E7xx_tier_parsing/E710_gra_invalid_index.cha`
**Trigger**: %gra relation with non-numeric index
**Expected Error Codes**: E600

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hello world .
%gra:	one|2|SUBJ 2|0|ROOT
@End
```

## Example 4

**Source**: `error_corpus/E7xx_tier_parsing/E711_gra_missing_role.cha`
**Trigger**: %gra relation with empty role field
**Expected Error Codes**: E342

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hello world .
%gra:	1|2| 2|0|ROOT
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: See example above

## CHAT Rule

See CHAT manual sections on main tier syntax and utterance structure. Every utterance must end with a terminator (., ?, or \!). The CHAT manual is available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
