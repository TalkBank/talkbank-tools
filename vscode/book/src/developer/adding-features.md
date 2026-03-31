# Adding Features

**Last updated:** 2026-03-30 13:40 EDT

This chapter describes the step-by-step process for adding a new LSP capability to the TalkBank language server. The pattern is consistent across all features.

## The 5-Step Process

### 1. Advertise the Capability

Edit `backend/capabilities.rs`. Add the new provider to `build_initialize_result()`:

```rust
// Example: adding rename support
rename_provider: Some(OneOf::Left(true)),
```

This tells the VS Code client that the server supports the new feature. Without this declaration, the client will never send requests for it.

### 2. Create the Feature Handler

Add a new file in `backend/features/` (e.g., `rename.rs`):

```rust
use lsp_types::*;
use crate::backend::state::Backend;

pub fn handle_rename(
    backend: &Backend,
    params: RenameParams,
) -> Option<WorkspaceEdit> {
    let uri = &params.text_document_position.text_document.uri;
    let text = backend.documents.get(uri)?;
    let chat_file = backend.chat_files.get(uri)?;
    // ... implementation using chat_file model data
}
```

Feature handlers are **pure functions**: they take `&Backend` and request parameters, and return an LSP response type. They do not mutate backend state (caches are updated only by the document lifecycle handlers in `documents.rs`).

### 3. Wire It into the LanguageServer Trait

Edit `backend/mod.rs`. Add the trait method implementation:

```rust
async fn rename(
    &self,
    params: RenameParams,
) -> Result<Option<WorkspaceEdit>> {
    Ok(features::rename::handle_rename(&self.backend, params))
}
```

### 4. Re-Export from the Module

Add the new module to `backend/features/mod.rs`:

```rust
pub mod rename;
```

### 5. Write Tests

Add tests in the feature file itself or in a dedicated test module. Because feature handlers are pure functions (`Backend` + params -> result), they are straightforward to test by constructing backend state and invoking handlers directly:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    // Construct a Backend with known document content,
    // call handle_rename(), assert the result.
}
```

## The Standard Handler Pattern

Every feature handler follows the same shape:

1. **Get document text** from `backend.documents`
2. **Get cached `ChatFile`** from `backend.chat_files` (or re-parse on cache miss)
3. **Get cached `Tree`** from `backend.parse_trees` (if tree-sitter node queries are needed)
4. **Compute the result** using the model data (`ChatFile`, `AlignmentSet`, etc.)
5. **Convert to LSP types** and return

This pattern keeps all domain logic in the core crates (`talkbank-model`, `talkbank-parser`) and limits the LSP layer to format conversion and routing.

## Adding a VS Code Command

If the new feature also needs a VS Code command (e.g., a button in the context menu):

### In `package.json`

Add the command definition:

```json
{
  "command": "talkbank.myFeature",
  "title": "My Feature",
  "category": "TalkBank"
}
```

Add keybindings, menu entries, and context conditions as needed.

### In `extension.ts`

Register the command handler:

```typescript
context.subscriptions.push(
  vscode.commands.registerCommand('talkbank.myFeature', async () => {
    // Send request to LSP, handle response, update UI
  })
);
```

### Rebuild

```bash
cd vscode && npm run compile
```

## Adding a Custom LSP Command

For features that do not map to standard LSP capabilities, use `workspace/executeCommand`. See [Custom Commands](custom-commands.md) for the list of existing custom commands and how to add new ones.

The process:

1. Add a command handler in the appropriate `.rs` file under `backend/`
2. Register the command name in `capabilities.rs` execute command options
3. Add dispatch logic in `backend/mod.rs` `execute_command()`
4. On the TypeScript side, send the command via `client.sendRequest('workspace/executeCommand', ...)`

## Related Chapters

- [Architecture](architecture.md) -- system design and module map
- [LSP Protocol](lsp-protocol.md) -- all advertised capabilities
- [Custom Commands](custom-commands.md) -- non-standard LSP commands
- [Testing](testing.md) -- how to test new features
