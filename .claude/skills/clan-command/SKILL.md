---
name: clan-command
description: Wire a new CLAN analysis command end-to-end, from talkbank-clan implementation through LSP and VS Code extension. Use when the user wants to add a new analysis command.
disable-model-invocation: true
allowed-tools: Bash, Read, Write, Edit, Glob, Grep, Agent
---

# Wire a New CLAN Analysis Command

Implement a CLAN analysis command end-to-end across all layers. The command name should be specified in `$ARGUMENTS` (e.g., `/clan-command newcmd`).

## Architecture

```
talkbank-clan (implement) → LSP analysis.rs (dispatch) → VS Code extension.ts (QuickPick) → analysisPanel.ts (display)
```

All code lives within this repo (`talkbank-tools`).

## Step 1: Implement the Command in talkbank-clan

Read the AnalysisCommand trait to understand the interface:

```bash
grep -n "trait AnalysisCommand" $REPO_ROOT/crates/talkbank-clan/src/lib.rs
```

Read an existing simple command as a template (e.g., `freq`):

```bash
ls $REPO_ROOT/crates/talkbank-clan/src/commands/
```

Create the new command module. Every command needs:

### Required Types
- `{Name}Config` — configuration struct (implements `Default` if sensible)
- `{Name}State` — per-file accumulator state
- `{Name}Output` — final result (must implement `Serialize`)
- `{Name}Command` — main struct implementing `AnalysisCommand`

### Required Trait Methods
```rust
impl AnalysisCommand for {Name}Command {
    type Config = {Name}Config;
    type State = {Name}State;
    type Output = {Name}Output;

    fn new(config: Self::Config) -> Self;           // or Result<Self, Error>
    fn init_state(&self) -> Self::State;
    fn process_utterance(&self, state: &mut Self::State, utterance: &Utterance);
    fn end_file(&self, state: &mut Self::State);     // optional, default no-op
    fn finalize(&self, state: Self::State) -> Self::Output;
}
```

### Register the Module
Add `pub mod {name};` to `commands/mod.rs`.

### Add Golden Snapshot Test
```bash
ls $REPO_ROOT/crates/talkbank-clan/tests/
```

Run tests:
```bash
cd $REPO_ROOT && cargo nextest run -p talkbank-clan
```

## Step 2: Wire into CLI (`args.rs` + `clan.rs`)

Add CLI subcommand in `crates/talkbank-cli/src/cli/args.rs` (`ClanCommands` enum).
Wire dispatch in `crates/talkbank-cli/src/commands/clan.rs` (`run_clan()`).

```bash
grep -n "ClanCommands" $REPO_ROOT/crates/talkbank-cli/src/cli/args.rs
grep -n "run_clan" $REPO_ROOT/crates/talkbank-cli/src/commands/clan.rs
```

## Step 3: Wire into LSP (`analysis.rs`)

**File:** `crates/talkbank-lsp/src/backend/analysis.rs`

### Add Import
```rust
use talkbank_clan::commands::{name}::{Name}Command, {Name}Config};
```

### Add Match Arm

Determine the constructor category:

**Simple (no user input needed):**
```rust
"{name}" => {
    let cmd = {Name}Command::new({Name}Config::default());
    run_json(&runner, &cmd, &files)
}
```

**Fallible constructor (built-in data files):**
```rust
"{name}" => {
    let cmd = {Name}Command::new({Name}Config { ... })
        .map_err(|e| format!("Failed to initialize {name}: {e}"))?;
    run_json(&runner, &cmd, &files)
}
```

**Requires user input (keywords, file paths):**
```rust
"{name}" => {
    let param = options
        .and_then(|o| o.get("paramName"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "{name} requires paramName option".to_string())?;
    let cmd = {Name}Command::new({Name}Config { param: param.into() });
    run_json(&runner, &cmd, &files)
}
```

**Two-file comparison (like `rely`):**
```rust
"{name}" => {
    let second_uri = options
        .and_then(|o| o.get("secondFile"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "{name} requires secondFile option (URI)".to_string())?;
    let second_url = Url::parse(second_uri).map_err(|e| format!("Invalid URI: {e}"))?;
    let second_path = second_url.to_file_path().map_err(|_| "Not a file path".to_string())?;
    let result = {name}::run_{name}(&config, &file_path, &second_path)
        .map_err(|e| format!("{e}"))?;
    serde_json::to_value(&result).map_err(|e| format!("Serialize error: {e}"))
}
```

Add the arm **before** the catch-all `_ => Err(...)` at the end of the match.

## Step 4: Wire into VS Code Extension (`extension.ts`)

**File:** `vscode/src/extension.ts`

### Add QuickPick Entry
Find the `analysisCommands` array and add:
```typescript
{ label: '{name}', description: 'One-line description of what the command does' },
```

### Add Input Prompts (if needed)
Find the existing input prompt blocks and add input for the command if it requires user parameters.

## Step 5: Add Display Info (`analysisPanel.ts`)

**File:** `vscode/src/analysisPanel.ts`

Find the `COMMAND_INFO` object and add:
```typescript
{name}: { label: 'Display Name', desc: 'Longer description for the panel header' },
```

## Step 6: Verify

Run these in order:

```bash
# CLAN crate compiles and passes lint
cd $REPO_ROOT && cargo clippy -p talkbank-clan --all-targets -- -D warnings

# CLAN tests pass
cd $REPO_ROOT && cargo nextest run -p talkbank-clan

# LSP compiles
cd $REPO_ROOT && cargo clippy -p talkbank-lsp -- -D warnings

# Full workspace tests pass
cd $REPO_ROOT && cargo nextest run --workspace

# VS Code extension compiles and lints
cd $REPO_ROOT/vscode && npm run compile && npm run lint
```

## Step 7: Report

Summarize:
- Command name and what it does
- Constructor category (simple / fallible / parameterized / two-file)
- Whether it needs user input prompts
- All verification results
