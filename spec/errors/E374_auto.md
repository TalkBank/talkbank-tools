# E374: ErrorAnnotationParseError

## Description

An error annotation (e.g. `[*]`) could not be parsed. This code was
declared for malformed error annotation content but the emission site
has not been wired up in the parser.

## Metadata

- **Error Code**: E374
- **Category**: parser\_recovery
- **Level**: utterance
- **Layer**: parser
- **Status**: not_implemented

## Notes

- Defined but not yet wired up. Intended for when `[*]`-style error
  annotations have malformed content.
- No code path currently emits this error.
