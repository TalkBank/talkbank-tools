# Testing

**Last updated:** 2026-03-30 13:40 EDT

This chapter covers how to test both the Rust language server and the TypeScript VS Code extension.

## Rust Language Server Tests

### Running Tests

```bash
cargo test -p talkbank-lsp
```

Or with the preferred parallel test runner:

```bash
cargo nextest run -p talkbank-lsp
```

### Test Locations

Tests are distributed across the LSP crate:

| Location | What It Tests |
|----------|--------------|
| `alignment/tests.rs` | Cross-tier alignment computation and hover info |
| `graph/tests.rs` | DOT graph generation from %gra data |
| Inline `#[cfg(test)]` modules | Individual feature handlers |

### Test Pattern

Feature handlers are pure functions (`&Backend` + params -> result), making them straightforward to test:

1. Construct a `Backend` with known document content loaded into the `documents` cache
2. Trigger a parse to populate `chat_files` and `parse_trees`
3. Call the handler function directly
4. Assert on the returned LSP type

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hover_on_main_tier_word() -> Result<()> {
        // 1. Set up backend with a known .cha document
        let backend = Backend::new_for_test();
        backend.load_document(uri, CHAT_CONTENT);

        // 2. Call the handler
        let result = handle_hover(
            &backend,
            HoverParams { /* position on a main tier word */ },
        );

        // 3. Assert
        assert!(result.is_some());
        let hover = result.unwrap();
        assert!(hover.contents.value.contains("%mor"));
        Ok(())
    }
}
```

## TypeScript Extension Tests

### Running Tests

```bash
cd vscode
npm test              # Run all tests (vitest)
npm run lint          # ESLint
npm run test:coverage # Tests with coverage report
```

### Test Framework

The extension uses [Vitest](https://vitest.dev/) for unit tests. Test files live in `src/test/`.

### What to Test

- **Utility functions** (`bulletParser.ts`, `alignmentSidecar.ts`, `mediaResolver.ts`) -- these are pure functions with no VS Code API dependencies
- **Panel message handling** -- test that PostMessage payloads are correctly formatted
- **Command argument construction** -- test that LSP command arguments are correctly assembled

VS Code API interactions (commands, webviews, tree views) are harder to unit test. For those, use manual testing.

## Manual Testing

### Launch Development Extension

```bash
cd vscode
code --extensionDevelopmentPath=.
```

This opens a new VS Code window with the extension loaded from source. Changes require recompiling (`npm run compile`) and reloading the window (`Cmd+Shift+P` > "Developer: Reload Window").

### Test Files

Use files from `corpus/reference/` for manual testing. This directory contains 87 CHAT files covering 20 languages and diverse CHAT constructs.

### Debugging the Language Server

Enable detailed logging by setting `RUST_LOG` before launching VS Code:

```bash
RUST_LOG=debug code --extensionDevelopmentPath=.
```

For targeted tracing of a specific module:

```bash
RUST_LOG=talkbank_lsp::alignment=trace code --extensionDevelopmentPath=.
```

Server output appears in the Output panel under "TalkBank Language Server".

### LSP Message Tracing

To inspect the raw JSON-RPC messages between VS Code and the server:

1. Check if the extension supports the `talkbank-lsp.trace.server` setting
2. Set it to `"verbose"` to log all request/response payloads
3. Alternatively, use VS Code's built-in LSP inspector (if available in your version)

## Coverage

```bash
cd vscode
npm run test:coverage
```

This generates a coverage report using Vitest's built-in coverage via `@vitest/coverage-v8`. Check the report to identify untested utility functions or edge cases.

## What Must Pass Before Committing

For LSP changes:

```bash
cargo nextest run -p talkbank-lsp        # LSP tests
cargo nextest run -p talkbank-parser-tests  # Parser equivalence (if touching parsing)
```

For extension changes:

```bash
cd vscode && npm test && npm run lint
```

For any change:

```bash
make verify    # Full verification gates (G0-G10)
```

## Related Chapters

- [Architecture](architecture.md) -- understanding the codebase structure
- [Adding Features](adding-features.md) -- testing new features
- [Troubleshooting: LSP Connection](../troubleshooting/lsp.md) -- debugging server issues
