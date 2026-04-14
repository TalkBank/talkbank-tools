# Installation

**Last updated:** 2026-04-13 20:34 EDT

This chapter walks you through installing VS Code, the TalkBank CHAT extension,
and the language server binary that powers it. By the end, you will have a
working environment where opening any `.cha` file gives you syntax highlighting,
real-time validation, and all the features described in this book.

---

## Prerequisites

You need two things:

1. **Visual Studio Code Insiders** version 1.110 or later
2. **The `talkbank-lsp` binary** (the language server that powers every feature)

The extension itself is a `.vsix` package installed into VS Code. It
communicates with `talkbank-lsp` over stdio using the Language Server Protocol.
The `talkbank-lsp` binary ships in the same release archive as the `chatter`
CLI — installing the release places both on your PATH.

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

## Step 2: Install the `talkbank-lsp` Binary

`talkbank-lsp` is the standalone Rust language server the extension spawns.
It ships in the same release archive as `chatter` (the CLI) — installing a
release gives you both binaries.

### From a release build

If you have a pre-built release archive, place both binaries somewhere on
your system PATH:

```bash
# Example: copy to /usr/local/bin
cp chatter talkbank-lsp /usr/local/bin/
talkbank-lsp --version
```

### Building from source

From the `talkbank-tools` repository root:

```bash
cargo build --release -p talkbank-lsp -p talkbank-cli
```

The binaries are at `target/release/talkbank-lsp` and `target/release/chatter`.
You can either add `target/release/` to your PATH or copy the binaries to a
standard location.

### Fleet machines (Ansible)

On TalkBank fleet machines, both `talkbank-lsp` and `chatter` are deployed
via Ansible. The deploy playbook builds the binaries and copies them to each
machine. You do not need to install them manually.

---

## Step 3: Install the Extension

### Option A: Ansible deployment (fleet machines)

On fleet machines, the extension `.vsix` is deployed automatically alongside
`talkbank-lsp` and `chatter`. No manual installation is needed.

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

When the extension starts, it searches for the `talkbank-lsp` binary in this order:

1. **`talkbank.lsp.binaryPath` setting** -- if you have set an explicit path in
   VS Code settings, that path is used directly
2. **System PATH** -- searches for `talkbank-lsp` using the standard `which` lookup
3. **`target/debug/talkbank-lsp`** -- relative to the extension directory (for
   developers running from the source tree)
4. **`target/release/talkbank-lsp`** -- relative to the extension directory

The binary is spawned with no arguments. It speaks the Language Server Protocol
over stdio as soon as it starts.

If none of these paths find a valid binary, you will see an error notification.
Set the `talkbank.lsp.binaryPath` setting to the absolute path of your
`talkbank-lsp` binary to resolve this. See [Settings Reference](../configuration/settings.md)
for details.

---

## Next Steps

With the extension installed, open a CHAT file and follow along with
[Your First CHAT File](first-file.md) to explore the core features.
