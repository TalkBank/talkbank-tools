# CI and Release

## Pre-Merge Verification

Every change must pass `make verify` before merging. This runs gates G0 through G10:

```bash
make verify
```

See [Testing > Verification Gates](testing.md#verification-gates) for the full gate table.

## Generated Artifact Check (G6)

`make generated-check` regenerates all artifacts and verifies they match what's committed:

- Symbol sets (Rust and JavaScript)
- Tree-sitter corpus tests
- Rust parser tests
- Error documentation

If the check fails, it means someone edited specs or symbols without running `make test-gen`.

## Parser Signature Guardrail (G7)

`make parser-guard` enforces a coding convention: parser functions should use consistent `ErrorSink` signatures. This prevents accidental introduction of incompatible parser APIs.

## Release Process

Releases are currently manual. The general process:

1. Ensure `make verify` passes on the release branch
2. Update version numbers in `Cargo.toml` files
3. Tag the release
4. Build downstream consumers (`batchalign3`)

## Cross-Repo Testing

After changes to core crates, verify the downstream consumer:

```bash
# batchalign (Python + Rust)
cd /path/to/batchalign3
uv run maturin develop    # Rebuild Rust extension
uv run pytest             # 878 tests
```

CLI, LSP, and CLAN tests run as part of the main workspace's `make test` and `make verify`.
