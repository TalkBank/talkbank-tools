# E318: UnparsableDependentTier

## Description

**NOT EMITTED.** This code was declared for dependent tier parse failures
but the implementation uses E316 (UnparsableContent) or more specific
error codes instead. Kept for backwards compatibility.

## Metadata

- **Error Code**: E318
- **Category**: parser\_recovery
- **Level**: utterance
- **Layer**: parser
- **Status**: not_implemented

## Notes

- Defined but never emitted. Dependent tier errors route to E316 or to
  specific tier error codes (e.g., E381 for %pho, E384 for %sin).
- No example is possible since no code path emits this error.
