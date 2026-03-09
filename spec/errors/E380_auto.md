# E380: UnknownSeparator

## Description

**NOT EMITTED.** This code was declared for unrecognized separator
characters between words but the emission site has not been wired up.
Separator parsing uses other code paths that don't emit this code.

## Metadata

- **Error Code**: E380
- **Category**: parser\_recovery
- **Level**: utterance
- **Layer**: parser
- **Status**: not_implemented

## Notes

- Defined but never emitted. Separator handling uses other error paths.
- No example is possible since no code path emits this error.
