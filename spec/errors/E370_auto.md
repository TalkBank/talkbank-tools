# E370: Structural order error

## Description

Structural order error

## Metadata

- **Error Code**: E370
- **Category**: Alignment count mismatch
- **Level**: utterance
- **Layer**: validation

## Example 1

**Source**: `E4xx_alignment_errors/multiple_replacements.cha`
**Trigger**: Multiple <original> [: corrected] forms
**Expected Error Codes**: E370

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|2;6|male|||Target_Child|||
@Comment:	Note: Each word in corrected form aligns separately
*CHI:	I <wanna> [: want to] <the> [: a] cookie .
%mor:	pro|I v|want n|cookie .
@Comment:	ERROR: Replacements have "want to" (2 words) and "a" (1 word)
@Comment:	Main tier alignable: I, want, to, a, cookie = 5 words
@Comment:	Mor tier: Should be pro|I v|want inf|to det|a n|cookie (5 items + terminator)
@End
```

## Example 2

**Source**: `E4xx_alignment_errors/group_alignment_mismatch.cha`
**Trigger**: Groups <like this> [: correction] have multiple alignable items
**Expected Error Codes**: E316

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|2;6|male|||Target_Child|||
@Comment:	Note: Group contents align recursively
*CHI:	I <want the> [: need the] cookie .
%mor:	pro|I v|need n|cookie .
@Comment:	ERROR: Replacement has 2 words in corrected form (need the)
@Comment:	Main tier alignable: I, need, the, cookie = 4 words
@Comment:	Mor tier: Should be pro|I v|need det|the n|cookie (4 items + terminator)
@End
```

## Example 3

**Source**: `E4xx_alignment_errors/complex_nested_structure.cha`
**Trigger**: Nested <groups [: corrections]> with multiple alignable items
**Expected Error Codes**: E316

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|2;6|male|||Target_Child|||
@Comment:	Note: Replacements can contain groups, which align recursively
*CHI:	I <want the big> [: need a large] cookie .
%mor:	pro|I v|need n|cookie .
@Comment:	ERROR: Replacement "need a large" has 3 words (need, a, large)
@Comment:	Main tier alignable: I, need, a, large, cookie = 5 words
@Comment:	Mor tier: Should be pro|I v|need det|a adj|large n|cookie (5 items + terminator)
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
