# LSP Connection

**Last updated:** 2026-04-13 20:34 EDT

The TalkBank extension is powered by a Rust language server (`talkbank-lsp`) that communicates with VS Code over stdio. If the language server fails to start or crashes, most extension features will not work. This chapter covers how to diagnose and fix LSP connection issues.

## Checking the Output Panel

The first step for any LSP issue is the Output panel:

1. Open the Output panel: **View > Output** or `Cmd+Shift+U` (macOS) / `Ctrl+Shift+U` (Windows/Linux)
2. Click the dropdown in the top-right of the Output panel
3. Select **"TalkBank Language Server"**

This shows the LSP communication logs and any server stderr output. Look for:

- **Startup messages** -- the server logs its version and capabilities on startup
- **Error messages** -- parse failures, missing files, or crashes
- **Connection lost** -- if the server crashed, you will see a disconnection message

## Binary Not Found

**Symptom:** The Output panel shows an error about not finding the `talkbank-lsp` binary.

The extension searches for the standalone language-server binary in three locations:

1. **System PATH** -- runs `which talkbank-lsp` (or `where talkbank-lsp` on Windows)
2. **`target/debug/talkbank-lsp`** -- relative to the extension directory (development builds)
3. **`target/release/talkbank-lsp`** -- relative to the extension directory (release builds)

**Fixes:**

- Build the binary: `cargo build -p talkbank-lsp` (debug) or `cargo build --release -p talkbank-lsp` (release)
- Or set `talkbank.lsp.binaryPath` in your settings to the absolute path of the `talkbank-lsp` binary
- Verify the binary exists and is executable: `which talkbank-lsp` or `ls -la target/release/talkbank-lsp`

## Enabling Trace Logging

For detailed server diagnostics, enable trace-level logging:

### Server-side logging (RUST_LOG)

Set the `RUST_LOG` environment variable before launching VS Code:

```bash
RUST_LOG=debug code .
```

Or for more targeted tracing:

```bash
RUST_LOG=talkbank_lsp=debug code .
RUST_LOG=talkbank_lsp::alignment=trace code .
```

The server uses the `tracing` crate for structured logging. Trace output appears in the Output panel under "TalkBank Language Server".

### LSP message inspection

To see the raw JSON-RPC messages between VS Code and the server, check whether the extension supports the `talkbank-lsp.trace.server` setting. Set it to `"verbose"` to log all request/response payloads. Alternatively, use VS Code's built-in LSP inspector if available.

## Server Crash Recovery

The extension automatically restarts the language server if it crashes. You should see a brief interruption in diagnostics and features, followed by automatic recovery.

If the server crashes repeatedly:

1. Check the Output panel for the crash message
2. Note the file you were editing when the crash occurred
3. Try opening a different `.cha` file to determine if the crash is file-specific
4. If the crash is reproducible, file a bug report with:
   - The `.cha` file that triggers the crash (or a minimal reproduction)
   - The Output panel log
   - The `RUST_LOG=debug` output

## Server Hangs (No Response)

**Symptom:** The language server appears to be running but features stop responding (no hover, no diagnostics, no completion).

**Possible causes:**

- **Large file parsing.** Very large CHAT files (thousands of utterances) can take time to parse. Wait a few seconds after opening or editing.
- **Debounce delay.** The server debounces validation by 250ms after each edit. During fast typing, diagnostics are intentionally delayed.
- **Deadlock.** Rare, but possible. Restart VS Code to recover. If reproducible, file a bug report with `RUST_LOG=debug` output.

## Verifying Server Version

To confirm which version of the language server is running, check the Output panel at startup. The server logs its version string. Compare this against the output of:

```bash
chatter --version
```

If the versions do not match, the extension may be using a stale binary. Rebuild or update `talkbank.lsp.binaryPath`.

## Related Chapters

- [Common Issues](common-issues.md) -- general troubleshooting
- [Settings Reference](../configuration/settings.md) -- `talkbank.lsp.binaryPath` setting
- [Architecture](../developer/architecture.md) -- how the LSP server works internally
