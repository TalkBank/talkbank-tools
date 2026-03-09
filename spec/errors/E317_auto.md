# E317: UnparsableFileContent

## Description

**NOT EMITTED.** This code was declared for top-level file parse failures
but the implementation uses E316 (UnparsableContent) instead. Kept for
backwards compatibility but never emitted by current parsers.

## Metadata

- **Error Code**: E317
- **Category**: parser\_recovery
- **Level**: utterance
- **Layer**: parser
- **Status**: not_implemented

## Notes

- Defined but never emitted. E316 is used for file-level parse failures.
- No example is possible since no code path emits this error.
