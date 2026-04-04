# E245 — Stress marker without following spoken material

**Status:** Current
**Last updated:** 2026-04-04 08:15 EDT

## Description

A primary stress marker (`ˈ`) or secondary stress marker appears at the start
of a word but is not followed by any spoken material. The marker has nothing
to attach to.

## Metadata

- **Error Code**: E245
- **Category**: validation
- **Level**: word
- **Layer**: validation
- **Status**: not_implemented

## Example 1

**Trigger**: Lone stress marker as entire word content
**Expected Error Codes**: E245

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
*CHI:	ˈ .
@End
```

## Expected Behavior

Validation should report E245 on the word `ˈ` because the stress marker has
no following spoken material.

**Known bug:** This example currently causes a panic in
`NonEmptyString::new_unchecked` because the parser strips the stress marker
and leaves an empty string for word content. The panic must be fixed before
this validation check can be implemented.

## CHAT Rule

See CHAT manual sections on stress markers and word-level syntax:
<https://talkbank.org/0info/manuals/CHAT.html>

## Notes

- The panic is tracked as a pre-existing parser bug (empty word content after
  stress marker stripping).
- E245 validation cannot be implemented until the panic is fixed.
