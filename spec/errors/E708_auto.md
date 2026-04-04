# E708: Malformed grammar relation on %gra tier

## Description

A grammar relation on the `%gra` tier is malformed — missing an index, head,
or relation label, or containing non-integer values where integers are expected.
The `%gra` tier format is `index|head|RELATION` for each word.

**Validation not yet implemented for this spec example.** The example has
`|2|SUBJ 2|0|ROOT` where the first relation has an empty index field. The
`MalformedGrammarRelation` check in `crates/talkbank-parser/src/parser/tier_parsers/gra/relation.rs`
does fire for missing or invalid indices, but the tree-sitter grammar may parse
`|2|SUBJ` differently than expected — the leading `|` may cause the grammar to
not recognize this as a GRA relation at all, producing a different error or
no error.

## Metadata
- **Status**: not_implemented
- **Last updated**: 2026-04-04 08:15 EDT
- **Layer**: validation

- **Error Code**: E708
- **Category**: Dependent tier parsing
- **Level**: tier
- **Layer**: validation

## Example 1

**Source**: `E7xx_tier_parsing/E709_gra_missing_index.cha`
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

## Expected Behavior

The parser should report E708 when a `%gra` relation is missing its index,
head, or relation label. The check exists in
`crates/talkbank-parser/src/parser/tier_parsers/gra/relation.rs`.

**Trigger conditions**: A `%gra` relation node in the CST where:
- The index child is missing or not a valid positive integer
- The head child is missing or not a valid non-negative integer
- The relation label child is missing or empty

## CHAT Rule

See CHAT manual on the `%gra` tier. Each relation must follow the format
`index|head|RELATION` where index and head are integers. The CHAT manual is
available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Validation logic exists in `gra/relation.rs` with multiple emission points
- The example has `|2|SUBJ` (missing index / leading pipe) which the grammar
  may not parse as a GRA relation node at all
- The code IS emitted in the codebase for malformed GRA relations that the
  grammar does recognize as relation nodes
