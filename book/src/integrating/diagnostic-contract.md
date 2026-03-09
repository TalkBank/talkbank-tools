# Diagnostic and JSON Output Contract

This page documents the machine-readable JSON surfaces currently exposed by the
top-level `chatter` CLI. It does not try to freeze every JSON payload emitted by
`chatter clan`; those command-specific formats should be checked per command.

## Stability policy

- Treat field names documented here as the public contract.
- Treat additional fields as additive unless this page says otherwise.
- Treat message wording as human-facing text, not a stable machine contract.

## `chatter validate FILE --format json`

Single-file validation emits one JSON object on stdout.

Observed shape:

```json
{
  "file": "/path/to/file.cha",
  "status": "valid",
  "cached": true,
  "error_count": 0,
  "errors": []
}
```

For invalid files, each error object currently includes:

```json
{
  "code": "E502",
  "severity": "Error",
  "message": "Missing required @End header",
  "location": {
    "start": 108,
    "end": 108
  }
}
```

Contract notes:

- `file`, `status`, `error_count`, and `errors` are stable.
- `cached` (optional) indicates the result came from the validation cache; only present when `true`.
- `status` is currently `valid` or `invalid`.
- `location.start` and `location.end` are byte offsets into the input file.
- Exit code `0` means success; exit code `1` means validation failure or I/O failure.

## `chatter validate DIR --format json`

Directory validation does not emit one aggregate JSON object. It currently emits
newline-delimited JSON records:

1. zero or more per-file records, then
2. one final summary record.

Observed record types:

```json
{
  "type": "file",
  "file": "/path/to/file.cha",
  "status": "valid",
  "cache_hit": false
}
```

```json
{
  "type": "file",
  "file": "/path/to/file.cha",
  "status": "invalid",
  "error_count": 1,
  "errors": [
    {
      "code": "E502",
      "message": "Missing required @End header",
      "severity": "Error"
    }
  ]
}
```

```json
{
  "type": "summary",
  "directory": "/path/to/dir",
  "total_files": 2,
  "valid": 1,
  "invalid": 1,
  "parse_errors": 0,
  "cache_hits": 0,
  "cache_misses": 2,
  "cache_hit_rate": 0.0,
  "cancelled": false
}
```

Contract notes:

- `type`, `file`, `status`, and the summary counters above are the stable fields.
- Directory-mode invalid-file errors currently omit `location`.
- Exit code `0` means all files validated successfully; exit code `1` means at least one file failed or an I/O error occurred.

## `chatter to-json`

`chatter to-json` emits the full `ChatFile` JSON model rather than a diagnostic
summary. The authoritative contract for that output is the JSON Schema
documented in [JSON Schema](json-schema.md).

Practical notes:

- The JSON itself is the contract, not any validation status lines printed by the CLI.
- Use `-o/--output` if you want only the JSON in a file.
- Use `--skip-validation`, `--skip-alignment`, or `--skip-schema-validation`
  only when you explicitly want to bypass those checks.

## `chatter cache stats --json`

Cache statistics emit one JSON object on stdout:

```json
{
  "total_entries": 743,
  "cache_dir": "/Users/example/Library/Caches/talkbank-chat",
  "cache_size_bytes": 274432,
  "last_modified": "2026-03-09T13:05:31+00:00"
}
```

Contract notes:

- `total_entries`, `cache_dir`, `cache_size_bytes`, and `last_modified` are stable.
- `last_modified` is RFC 3339 / ISO 8601 text.

## `chatter clan ... --format json`

CLAN analysis commands such as `chatter clan freq --format json` also provide
JSON output, but those shapes are command-specific. Do not assume one shared
schema across all CLAN commands. Check the command-specific documentation or
the command's `--help` output when you build an integration.
