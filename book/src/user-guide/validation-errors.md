# Validation Errors

**Status:** Current
**Last updated:** 2026-03-24 00:01 EDT

The CHAT validator produces diagnostics at two severity levels: **errors** (must fix) and **warnings** (should fix). Each diagnostic has an error code that links to detailed documentation.

## Reading Error Output

```
corpus/sample.cha:15:3: error[E304]: missing speaker code on main tier line
corpus/sample.cha:20:1: error[E308]: speaker MOT not declared in @Participants
```

Each line contains:
- **File path** and **location** (line:column)
- **Severity** — `error` or `warning`
- **Error code** — `E` prefix for errors, `W` prefix for warnings
- **Message** — human-readable description

## Error Code Ranges

| Range | Category | Examples |
|-------|----------|----------|
| E1xx | UTF-8 and encoding | E101: Invalid line format |
| E2xx | File structure | E202: Missing @Begin, E203: Missing @End |
| E3xx | Main tier (speakers, terminators, content) | E304: Missing speaker, E308: Undeclared speaker, E305: Missing terminator |
| E4xx | Dependent tier structure | E401: Duplicate dependent tier |
| E5xx | Headers | E501: Duplicate header, E504: Missing @Participants, E505: Invalid @ID format |
| E6xx | Dependent tier validation | E601: Invalid dependent tier, E604: %gra without %mor |
| E7xx | Alignment (`%mor`, `%gra`, `%pho`, `%wor`) | E705: Main/%mor count mismatch, E721: %gra index error |
| W1xx-W6xx | Warnings | W108: BOM detected, W601: Empty user-defined tier |

## Common Errors and Fixes

### E304: Missing speaker code

A main tier line must have a speaker code after the `*`:

```
*CHI:	hello world .
```

An empty speaker code (`*:	hello .`) triggers E304.

### E308: Undeclared speaker

Every `*SPEAKER:` code must be listed in `@Participants`. Add the missing speaker to the header:

```
@Participants:	CHI Target_Child, MOT Mother
```

### E505: Invalid @ID format

Check that pipe-separated fields are correct and the speaker code matches `@Participants`:

```
@ID:	eng|corpus|CHI|2;6.||||Target_Child|||
```

### E705: Main/%mor alignment mismatch

The number of `%mor` items must match the number of alignable words on the main tier. Retraces, pauses, and events are not counted. The validator shows a columnar diff:

```
  Main tier       %mor tier
  ──────────────  ──────────────
  I               pro|I
  want            v|want
  to              inf|to
  go              v|go
  home            — ⊖
```

### E721: %gra sequential index error

`%gra` entries must have sequential 1-based indices: `1|...|... 2|...|... 3|...|...`

## Generated Error Documentation

Detailed documentation for every error code is auto-generated from the spec at `docs/errors/`. Each page includes:
- Error description
- Example input that triggers the error
- Suggested fix
- Which validation layer catches it (parser vs. validation)

Run `make test-gen` to regenerate error documentation after **error spec**
changes.
