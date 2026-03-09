# E384: SinParseError

## Description

The `%sin` tier content could not be parsed by the direct parser (chumsky).
The sentence-internal tier content does not match the expected format.

## Metadata

- **Error Code**: E384
- **Category**: tier\_parse
- **Level**: utterance
- **Layer**: parser
- **Status**: not_implemented

## Example 1

**Trigger**: Empty sin group marker

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|test|CHI||female|||Target_Child|||
*CHI:	hello .
%sin:	〔〕
@End
```

## Expected Behavior

The direct parser should report E384 when the `%sin` tier content cannot
be parsed. A `〔〕` group must contain at least one token.

## Notes

- This error is emitted by the **direct parser** (chumsky) only, not by
  the tree-sitter canonical parser.
- The `%sin` tier uses `〔〕` (fullwidth tortoise shell brackets) as group
  delimiters, distinct from the `‹›` used by `%pho`.
