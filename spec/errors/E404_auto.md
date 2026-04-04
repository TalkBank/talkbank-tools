# E404: Orphaned dependent tier

**Last updated:** 2026-04-04 08:28 EDT

## Description

A dependent tier (`%mor`, `%gra`, etc.) appears before any main tier in the
file. E404 (OrphanedDependentTier) is emitted by
`report_top_level_dependent_tier_error()` in `helpers.rs` when a `%`-prefixed
ERROR node appears before any utterance. However, a `%mor:` line immediately
after headers causes the tree-sitter grammar to catastrophically fail,
producing header validation errors instead of E404.

## Metadata
- **Status**: not_implemented

- **Error Code**: E404
- **Category**: validation
- **Level**: tier
- **Layer**: validation

## Example 1

**Source**: `error_corpus/validation_errors/E404_orphaned_dependent_tier.cha`
**Trigger**: `%mor:` tier appears before any `*SPK:` main tier. The
tree-sitter grammar's error recovery breaks down: the orphaned dependent tier
causes the parser to fail to recognize ALL preceding headers, producing
cascading header validation errors.
**Expected Error Codes**: E501, E502, E503, E504, E505

Note: E404 IS emitted in the parser for orphaned dependent tiers, but only
when the tree-sitter grammar successfully parses the file structure around the
orphaned tier (producing an ERROR node that starts with `%`). In this example,
the orphaned `%mor:` right after headers causes a catastrophic parse failure
where no headers are recognized at all, so E501-E505 fire instead.

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

- E404 IS emitted in real-world parsing (when the grammar's error recovery
  handles the orphaned tier as an ERROR node), but the specific example here
  causes tree-sitter to fail too severely for E404 to fire.
- The cascading failure from a `%mor:` immediately after headers causes the
  parser to miss all headers, producing E501-E505 instead.
