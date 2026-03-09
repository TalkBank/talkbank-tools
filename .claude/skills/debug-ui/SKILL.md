---
name: debug-ui
description: Debug VS Code extension webviews, TUI (ratatui), and LSP diagnostic issues. Symptom-based triage for UI problems in talkbank-tools.
disable-model-invocation: true
allowed-tools: Bash, Read, Glob, Grep, Agent
---

# UI Debugging — VS Code, TUI, and LSP

Diagnose UI problems in talkbank-tools interfaces. `$ARGUMENTS` describes the symptom.

## Step 1: Identify Which UI System

| If the problem is in... | System | Key Files |
|------------------------|--------|-----------|
| VS Code (analysis panel, graph, media, waveform) | VS Code Webview | `vscode/src/` |
| Terminal (`chatter validate --interactive`) | ratatui TUI | `crates/talkbank-cli/src/ui/` |
| VS Code showing wrong errors / missing diagnostics | LSP Diagnostics | `crates/talkbank-lsp/src/backend/` |

## Step 2: Symptom-Based Triage

### "Blank screen / nothing renders"

**VS Code webview:**
- Is `enableScripts: true` set? (required for any JS)
- Check Content Security Policy `<meta>` tag
- Right-click webview → "Developer: Open Webview Developer Tools" → Console for JS errors
- Check if LSP returned data: Output panel → "TalkBank Language Server"

**TUI:**
- Is terminal alternate screen active? Try `reset` to restore
- Check terminal size: ratatui needs minimum dimensions
- Check TERM env var: `echo $TERM` (needs xterm-256color or similar)

---

### "Data not updating / stale data"

**VS Code webview:**
- Webview gets data at creation time — no auto-refresh
- Save the document to trigger re-validation by LSP
- Close and reopen the panel to force refresh

**TUI:**
- Streaming mode auto-updates; static mode is snapshot-only
- Press `r` to rerun validation

**LSP diagnostics:**
- Check if the LSP server is running: VS Code Output panel → "TalkBank Language Server"
- Check if the file was saved (LSP re-validates on save)
- Restart LSP: Command Palette → "TalkBank: Restart Language Server"

---

### "Layout broken / elements overlapping"

**VS Code webview:**
- Check viewport meta tag: `<meta name="viewport" content="width=device-width, initial-scale=1.0">`
- Check CSS for absolute positioning without containment
- Test in both narrow and wide panel widths

**TUI:**
- Terminal too small? ratatui constraints may underflow
- Use `Constraint::Min()` for flexible regions
- Check for hardcoded `Length()` values that don't fit

---

### "Wrong colors / unreadable text"

**VS Code webview:**
- Using hardcoded colors instead of `var(--vscode-*)` CSS variables?
- Test with both dark and light VS Code themes
- Check: `var(--vscode-foreground)`, `var(--vscode-editor-background)`

**TUI:**
- Terminal color support varies (8, 256, truecolor)
- Check `$COLORTERM` env var (should be `truecolor` for RGB colors)

---

### "Keyboard/mouse not working"

**VS Code webview:**
- Webview captures focus — extension keyboard shortcuts may not work inside webview
- Use `retainContextWhenHidden: true` if panel should keep state when tabbed away

**TUI:**
- Check key bindings in the TUI module
- Raw mode must be enabled (`enable_raw_mode()`)
- Mouse events not implemented — keyboard only

---

### "Performance / lag"

**VS Code webview:**
- Large DOT graphs cause Graphviz WASM lag → consider limiting graph size
- Waveform canvas redraws on every scroll → throttle with `requestAnimationFrame`

**TUI:**
- `poll(Duration::from_millis(100))` throttles to 10 FPS — increase interval if still slow
- Avoid blocking operations in the render loop

---

### "Media won't play" (VS Code only)

- Check `@Media:` header exists in .cha file
- Check file exists at resolved path (media resolver tries multiple locations)
- Check `localResourceRoots` includes the media file's directory
- Check `asWebviewUri()` conversion is applied to the path
- Check CSP allows media: `media-src ${webview.cspSource}`
- Check audio format is browser-supported (WAV, MP3, OGG — not all codecs)

---

### "Wrong/missing validation diagnostics" (LSP)

- Check error code in spec: `ls $REPO_ROOT/spec/errors/ | grep -i "E<NNN>"`
- Check if validation rule is implemented: `grep -rn "E<NNN>" $REPO_ROOT/crates/talkbank-model/src/validation/`
- Check parser output: `cargo run -p talkbank-cli -- validate <file.cha>`
- Compare with LSP output in VS Code Output panel
- LSP uses streaming parse + incremental re-validation — timing issues possible

## Step 3: Build Verification

```bash
# VS Code extension
cd $REPO_ROOT/vscode && npm run compile 2>&1 | head -20

# TUI (Rust)
cd $REPO_ROOT && cargo check -p talkbank-cli 2>&1 | head -20

# LSP
cd $REPO_ROOT && cargo check -p talkbank-lsp 2>&1 | head -20
```

## Key Files

| Purpose | Path |
|---------|------|
| VS Code extension entry | `vscode/src/extension.ts` |
| Analysis panel webview | `vscode/src/analysisPanel.ts` |
| Media player webview | `vscode/src/mediaPanel.ts` |
| LSP backend | `crates/talkbank-lsp/src/backend/` |
| LSP analysis dispatch | `crates/talkbank-lsp/src/backend/analysis.rs` |
| TUI validation | `crates/talkbank-cli/src/ui/` |
| VS Code package manifest | `vscode/package.json` |
