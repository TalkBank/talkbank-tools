# E378: OverlapAnnotationParseError

## Description

An overlap annotation (`[<]`, `[>]`) could not be parsed. This code was
declared for malformed overlap marker content but the emission site has
not been wired up in the parser.

## Metadata

- **Error Code**: E378
- **Category**: parser\_recovery
- **Level**: utterance
- **Layer**: parser
- **Status**: not_implemented

## Notes

- Defined but not yet wired up. Intended for when overlap markers have
  malformed content.
- No code path currently emits this error.
