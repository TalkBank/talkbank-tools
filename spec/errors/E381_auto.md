# E381: PhoParseError

## Description

The `%pho` tier content could not be parsed by the direct parser (chumsky).
The phonological tier content does not match the expected format of
space-separated phonological words with optional `‹›` groups.

## Metadata

- **Error Code**: E381
- **Category**: tier\_parse
- **Level**: utterance
- **Layer**: parser
- **Status**: not_implemented

## Example 1

**Trigger**: Unclosed phonological group marker

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|test|CHI||female|||Target_Child|||
*CHI:	hello .
%pho:	‹hɛloʊ
@End
```

## Example 2

**Trigger**: Empty phonological group

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|test|CHI||female|||Target_Child|||
*CHI:	hello .
%pho:	‹›
@End
```

## Expected Behavior

The direct parser should report E381 when the `%pho` tier content cannot
be parsed. The `‹` group delimiter must be properly closed with `›`, and
groups must contain at least one phonological word.

## Notes

- This error is emitted by the **direct parser** (chumsky) only, not by
  the tree-sitter canonical parser.
- The tree-sitter grammar handles %pho differently and may not produce
  the same error for the same input.
