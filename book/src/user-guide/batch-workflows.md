# Batch Workflows

The `chatter` CLI is designed for processing large CHAT corpora efficiently. This page covers common batch workflows.

## Validating a Corpus

Validate all `.cha` files in a directory tree:

```bash
chatter validate /path/to/corpus/
```

The validator recursively discovers `.cha` files and processes them in parallel. Results are cached — subsequent runs skip unchanged files.

### Forcing Revalidation

To bypass the cache and revalidate everything:

```bash
chatter validate /path/to/corpus/ --force
```

### Filtering Output

Show only errors (hide warnings):

```bash
chatter validate /path/to/corpus/ --quiet
```

Stop after the first reported error:

```bash
chatter validate /path/to/corpus/ --max-errors 1
```

Write a JSONL audit file while validating:

```bash
chatter validate /path/to/corpus/ --audit validation.jsonl
```

## CHAT-JSON Roundtrip

Convert an entire corpus to JSON and back:

```bash
# CHAT → JSON
for f in corpus/**/*.cha; do
  chatter to-json "$f" > "${f%.cha}.json"
done

# JSON → CHAT
for f in corpus/**/*.json; do
  chatter from-json "$f" > "${f%.json}.roundtrip.cha"
done
```

The roundtrip is designed to preserve the `ChatFile` model. In regression
tests, compare normalized output rather than assuming byte-for-byte identity
after parser or serializer changes.

## Cache Management

The validation cache at `~/.cache/talkbank-chat/talkbank-cache.db` stores results for previously validated files (keyed by content hash). It can hold results for large file collections.

```bash
chatter cache stats    # Show hit rates and entry count
chatter cache clear --all
```

Do not delete the cache file manually while `chatter` is running.

## Reference Corpus Validation

The `talkbank-tools` repository includes a reference corpus of 74 files at `corpus/reference/`. Both parsers must agree on these files at 100%:

```bash
cargo nextest run -p talkbank-parser-tests -E 'test(parser_equivalence)'
```

This runs the parser equivalence test — each `.cha` file is its own test, so nextest runs them in parallel and reports individual failures.

## Integration with batchalign

The [batchalign3](https://github.com/talkbank/batchalign3) pipeline uses the same Rust core (via PyO3) for CHAT parsing and serialization. Files processed by batchalign produce valid CHAT that passes `chatter validate`.
