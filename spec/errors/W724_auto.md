# W724: GRA ROOT head not self

**Last updated:** 2026-04-04 08:15 EDT

## Description

ROOT relation where the head index does not point to self (i.e., `head != index` and `head != 0`). This warning code (`GraRootHeadNotSelf`) is defined in the error code enum but is never emitted by any validation code path.

## Metadata

- **Error Code**: W724
- **Category**: validation
- **Level**: tier
- **Layer**: validation
- **Status**: not_implemented

## Example 1

**Source**: `error_corpus/warnings/W724_gra_root_head_not_self.cha`
**Trigger**: ROOT relation where head index does not point to self — produces E722 (no root) and E724 (circular dependency) instead
**Expected Error Codes**: E722

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
*CHI:	I want .
%mor:	pro|I v|want .
%gra:	1|2|SUBJ 2|1|ROOT
@End
```

## Expected Behavior

The validator reports E722 (no ROOT) because `2|1|ROOT` has `head=1` which is neither `head=0` nor `head=self(2)`, so it is not recognized as a ROOT by the structural validator.

**Trigger**: See example above

## CHAT Rule

See the CHAT manual for format specifications: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- The W724 error code (`GraRootHeadNotSelf`) is defined in `error_code.rs` but is never emitted by any validation or parser code path. The `validate_gra_structure` function in `gra/tier.rs` checks for roots (E722/E723) and circular dependencies (E724) but does not emit W724. The example input triggers E722 (no root found, since `head=1 != self=2` is not recognized as ROOT) and E724 (circular dependency: 1->2->1).
- The previous example had a trailing ` .` on the `%gra` tier which caused tree-sitter to produce E316. `%gra` tiers do not have terminators.
- Review and enhance this specification as needed
