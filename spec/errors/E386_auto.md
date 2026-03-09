# E386: TextTierParseError

## Description

**NOT EMITTED.** This code was declared for text-based dependent tier
parse failures (e.g. `%com`, `%exp`) but the direct parser's `text_tier.rs`
uses `ParseOutcome::rejected()` on error without emitting this code.

## Metadata

- **Error Code**: E386
- **Category**: tier\_parse
- **Level**: utterance
- **Layer**: parser
- **Status**: not_implemented

## Notes

- Defined but never emitted. The direct parser silently rejects
  malformed text tiers instead of reporting E386.
- No example is possible since no code path emits this error.
