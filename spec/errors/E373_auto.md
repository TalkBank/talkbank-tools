# E373: InvalidOverlapIndex

## Description

An overlap marker has an index value outside the valid range. For CA
overlap brackets (`⌈⌉⌊⌋`), the index must be 2–9. For scoped overlap
annotations (`[<]`, `[>]`), the index must be 1–9.

## Metadata
- **Status**: not_implemented

- **Error Code**: E373
- **Category**: overlap
- **Level**: utterance
- **Layer**: validation

## Example 1

**Trigger**: Scoped overlap annotation with index 0

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	MOT Mother, CHI Target_Child
@ID:	eng|test|MOT||female|||Mother|||
@ID:	eng|test|CHI||male|||Target_Child|||
*MOT:	I think [<0] so .
*CHI:	yeah [>0] .
@End
```

## Example 2

**Trigger**: Scoped overlap annotation with index exceeding 9

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	MOT Mother, CHI Target_Child
@ID:	eng|test|MOT||female|||Mother|||
@ID:	eng|test|CHI||male|||Target_Child|||
*MOT:	I think [<10] so .
*CHI:	yeah [>10] .
@End
```

## Expected Behavior

Validation should report E373. Overlap indices must be within valid ranges:
1–9 for scoped overlap annotations, 2–9 for CA overlap point brackets.

## CHAT Rule

Overlap markers use numeric indices to pair overlapping speech segments
between speakers. Valid indices are single digits (1–9 for scoped, 2–9
for CA brackets). See the CHAT manual on overlap conventions.
