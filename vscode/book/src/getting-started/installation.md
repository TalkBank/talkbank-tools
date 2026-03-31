# Installation

**Last updated:** 2026-03-30 13:40 EDT

This chapter walks you through installing VS Code, the TalkBank CHAT extension,
and the language server binary that powers it. By the end, you will have a
working environment where opening any `.cha` file gives you syntax highlighting,
real-time validation, and all the features described in this book.

---

## Prerequisites

You need two things:

1. **Visual Studio Code Insiders** version 1.110 or later
2. **The `chatter` binary** (the language server that powers every feature)

The extension itself is a `.vsix` package installed into VS Code. It
communicates with `chatter` over stdio using the Language Server Protocol.

---

## Step 1: Install VS Code Insiders

### macOS

Install via Homebrew:

```bash
brew install --cask visual-studio-code@insiders
```

This gives you the `code-insiders` command in your terminal. Fleet machines
(net, brian, bilbo, etc.) receive VS Code Insiders automatically via Ansible
provisioning.

### Windows

Download from <https://code.visualstudio.com/insiders/>. Run the installer and
ensure "Add to PATH" is checked so the `code-insiders` command is available from
the terminal.

### Linux

Download the `.deb` or `.rpm` package from
<https://code.visualstudio.com/insiders/>, or use snap:

```bash
sudo snap install code-insiders --classic
```

---

## Step 2: Install the `chatter` Binary

The `chatter` binary is the Rust CLI that includes the language server
(`chatter lsp`). The extension launches it automatically.

### From a release build

If you have a pre-built `chatter` binary, place it somewhere on your system
PATH:

```bash
# Example: copy to /usr/local/bin
cp chatter /usr/local/bin/
chatter --version
```

### Building from source

From the `talkbank-tools` repository root:

```bash
cargo build --release -p talkbank-cli
```

The binary is at `target/release/chatter`. You can either add `target/release/`
to your PATH or copy the binary to a standard location.

### Fleet machines (Ansible)

On TalkBank fleet machines, `chatter` is deployed via Ansible. The deploy
playbook builds the binary and copies it to each machine. You do not need to
install it manually.

---

## Step 3: Install the Extension

### Option A: Ansible deployment (fleet machines)

On fleet machines, the extension `.vsix` is deployed automatically alongside
`chatter`. No manual installation is needed.

### Option B: Manual `.vsix` install

If you have a `.vsix` package file:

```bash
code-insiders --install-extension talkbank-chat-0.1.0.vsix
```

Or from inside VS Code: open the Command Palette (`Cmd+Shift+P`), type
**"Extensions: Install from VSIX..."**, and select the `.vsix` file.

### Option C: Development mode (building from source)

```bash
# From the talkbank-tools/ repository root:
cd vscode
npm install
npm run compile

# Launch VS Code with the extension loaded in development mode:
code-insiders --extensionDevelopmentPath=.
```

---

## Step 4: Verify the Installation

1. Open VS Code Insiders
2. Open any `.cha` file (if you have the repository, try one from
   `corpus/reference/`)
3. You should see:
   - **Syntax highlighting** -- headers, speaker codes, dependent tiers, and
     annotations are colored by category
   - **Validation diagnostics** -- any errors or warnings appear as squiggly
     underlines and in the Problems panel (`Cmd+Shift+M`)
   - **"CHAT Validation" tree view** in the Explorer sidebar

> **[SCREENSHOT: VS Code Insiders with a CHAT file open]**
> *Capture this: a `.cha` file from `corpus/reference/` open in the editor,
> showing syntax highlighting, the Problems panel with diagnostics, and the
> CHAT Validation tree view in the Explorer sidebar.*

If you see plain, uncolored text, the extension did not activate. Check:

- The file has a `.cha` extension
- The language mode in the bottom status bar says "CHAT" (not "Plain Text")
- The Output panel (`Cmd+Shift+U`) shows a "TalkBank CHAT" channel with server
  startup messages

---

## How the LSP Binary Is Found

When the extension starts, it searches for the `chatter` binary in this order:

1. **`talkbank.lsp.binaryPath` setting** -- if you have set an explicit path in
   VS Code settings, that path is used directly
2. **System PATH** -- searches for `chatter` using the standard `which` lookup
3. **`target/debug/chatter`** -- relative to the workspace root (for developers
   running from the source tree)
4. **`target/release/chatter`** -- relative to the workspace root

The extension launches the binary with the `lsp` subcommand:
`chatter lsp`. This starts the language server, which communicates with VS Code
over stdio.

If none of these paths find a valid binary, you will see an error notification.
Set the `talkbank.lsp.binaryPath` setting to the absolute path of your
`chatter` binary to resolve this. See [Settings Reference](../configuration/settings.md)
for details.

---

## Next Steps

With the extension installed, open a CHAT file and follow along with
[Your First CHAT File](first-file.md) to explore the core features.
