# Releasing

**Status:** Current
**Last updated:** 2026-04-16 16:19 EDT

The extension is published as five platform-specific VSIX files via the
`VS Code Extension Release` workflow
(`.github/workflows/vscode-release.yml`). Each VSIX bundles a prebuilt
`talkbank-lsp` for its target platform at
`extension/server/talkbank-lsp[.exe]`, so end users never need a Rust
toolchain or a PATH binary — the extension finds the bundled server
automatically (see [LSP Binary Discovery](../troubleshooting/lsp.md)).

## Cutting a release

1. Bump `vscode/package.json` version.
2. Commit and push to `main`.
3. Trigger the workflow:
   ```bash
   gh workflow run vscode-release.yml -f version=X.Y.Z
   ```
4. Wait for all five matrix jobs to succeed:
   - `darwin-arm64`
   - `darwin-x64`
   - `linux-x64`
   - `linux-arm64`
   - `win32-x64`
5. The workflow creates a GitHub Release tagged `vscode-vX.Y.Z` with the
   five VSIX files attached.

## Local packaging (single platform)

For a manual build targeting your current host:

```bash
cd <talkbank-tools-root>
cargo build --release -p talkbank-lsp
cd vscode
node scripts/prepare-server.mjs --source ../target/release/talkbank-lsp
npm run package:darwin-arm64   # or darwin-x64, linux-x64, linux-arm64, win32-x64
```

`prepare-server.mjs` stages the binary into `vscode/server/` with the
correct name (`talkbank-lsp` on Unix, `talkbank-lsp.exe` on Windows) so
`vsce package` bundles it at `<extension>/server/` inside the VSIX.

## Users installing a VSIX

1. Go to <https://github.com/TalkBank/talkbank-tools/releases/latest>.
2. Download the VSIX matching the user's platform.
3. Install from the command line:
   ```bash
   code --install-extension talkbank-chat-<platform>.vsix
   ```

No Rust toolchain, no PATH setup, no extra dependencies.

## Version number discipline

`vscode/package.json` is the single source of truth for the extension
version. Do not hand-edit the `extension/server/talkbank-lsp` binary
name or the tag format — the CI workflow derives both from `version`.
For a release candidate, append a pre-release identifier
(`0.X.Y-rc.1`) per semver; VS Code Marketplace treats those as
pre-release channels.

## Related chapters

- [Installation](../getting-started/installation.md) — user-facing install
- [Troubleshooting: LSP Connection](../troubleshooting/lsp.md) — binary discovery order
- [Testing](testing.md) — the test gates a release must pass
