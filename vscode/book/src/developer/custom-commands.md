# Custom Commands

**Last updated:** 2026-03-30 13:40 EDT

The TalkBank language server defines 12 custom LSP commands invoked via `workspace/executeCommand`. These commands provide functionality beyond standard LSP capabilities -- analysis execution, document filtering, graph generation, and more.

## Command Reference

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `talkbank/showDependencyGraph` | `[fileUri, line]` | DOT string | Generate a Graphviz DOT dependency graph for the utterance at the given line. The extension renders the DOT string to SVG via bundled Graphviz WASM. |
| `talkbank/analyze` | `[fileUri, command, args]` | JSON | Run a CLAN analysis command (one of 33) on the file. The `command` is a string like `"freq"`, `"mlu"`, etc. Optional `args` for commands that need user input. |
| `talkbank/getParticipants` | `[fileUri]` | `IdEntry[]` | Parse all `@ID` lines in the document into structured fields (10 pipe-delimited components: language, corpus, speaker, age, sex, group, SES, role, education, custom). Used by the ID Editor panel. |
| `talkbank/formatIdLine` | `[fields]` | string | Serialize structured `IdEntry` fields back to a canonical `@ID:` line. The inverse of `getParticipants`. |
| `talkbank/kidevalDatabases` | `[libDir]` | JSON | Discover available KidEval normative database files in the given directory. Returns `AvailableDatabase[]` with `language`, `corpus_type`, `entry_count`, and `path` — used by the KidEval panel's language/activity selector and availability grid. |
| `talkbank/evalDatabases` | `[libDir]` | JSON | Discover available Eval normative database files. Same response shape as `kidevalDatabases`, used by the Eval/Eval-D panels. |
| `talkbank/getSpeakers` | `[fileUri]` | `string[]` | Extract declared speaker codes from the document's `@Participants` header. Used by the speaker filter QuickPick. |
| `talkbank/filterDocument` | `[fileUri, speakers[]]` | string | Filter the document to include only headers and utterance blocks from the selected speakers. Returns the filtered document text for the virtual document. |
| `talkbank/getUtterances` | `[fileUri]` | `Utterance[]` | Extract all utterances with speaker code and timing information. Used by Coder Mode to find uncoded utterances and by Review Mode to find flagged utterances. |
| `talkbank/formatBulletLine` | `[timestamp]` | string | Format a timing bullet for transcription mode. Takes a timestamp and returns the formatted bullet string for insertion. |
| `talkbank/scopedFind` | `[ScopedFindInput]` | `ScopedFindMatch[]` | Search within specific tiers (main, %mor, %gra, %pho, %sin, %act, %cod, %com, %exp, or all) and optionally filter by speaker. Supports plain text and regex (prefix query with `/`). |
| `talkbank/getAlignmentSidecar` | `[fileUri]` | `AlignmentData` | Get per-utterance timing alignment data for media playback. Returns segments in document order with begin/end timestamps. Used by the media panel for playback coordination. |

## Architecture: Why Custom Commands?

Custom commands are used when the functionality does not fit a standard LSP capability. The standard capabilities (hover, completion, rename, etc.) have fixed request/response shapes defined by the LSP specification. Custom commands allow arbitrary JSON parameters and return types.

All 12 commands follow the same pattern:

1. **TypeScript sends** `workspace/executeCommand` with the command name and parameters
2. **Rust dispatches** in `backend/mod.rs` `execute_command()` to the appropriate handler
3. **Handler computes** the result using the `ChatFile` model (no string parsing in TypeScript)
4. **JSON response** is returned to TypeScript for rendering

This keeps all CHAT parsing and domain logic in Rust. TypeScript is a thin UI layer.

## Usage from TypeScript

```typescript
// Example: getting speakers for the filter picker
const speakers = await client.sendRequest(
  'workspace/executeCommand',
  {
    command: 'talkbank/getSpeakers',
    arguments: [document.uri.toString()]
  }
);
```

## Adding a New Custom Command

1. **Create the handler** in the appropriate `backend/*.rs` file
2. **Register the command name** in `capabilities.rs` under the execute command options list
3. **Add dispatch** in `backend/mod.rs` `execute_command()` to route the command name to the handler
4. **Add the TypeScript call** in the relevant extension command handler
5. **Write tests** for both the Rust handler and the TypeScript integration

## Related Chapters

- [Architecture](architecture.md) -- system design showing how commands flow
- [LSP Protocol](lsp-protocol.md) -- standard LSP capabilities
- [Adding Features](adding-features.md) -- general feature addition process
