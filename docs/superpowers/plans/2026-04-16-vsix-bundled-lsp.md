# VSIX-Bundled talkbank-lsp Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status:** Draft
**Last updated:** 2026-04-16 12:55 EDT

**Goal:** Publish per-platform VSIX files for the TalkBank CHAT VS Code extension that bundle a prebuilt `talkbank-lsp` binary, so users who install the VSIX never need to build from source.

**Architecture:** Match the rust-analyzer model. For each supported platform, CI builds `talkbank-lsp`, copies it into `vscode/server/`, then calls `vsce package --target <platform>` to produce a platform-specific VSIX (e.g. `talkbank-chat-darwin-arm64-0.1.0.vsix`). The extension's discovery logic checks the bundled binary first, then falls back to PATH and dev-tree paths (the latter only for contributors running from a cloned workspace). A proper `.vscodeignore` shrinks the VSIX by excluding `node_modules` dev deps, tests, and docs.

**Tech Stack:**
- Rust (cross-compiled) — `talkbank-lsp` binary, 5 target triples
- TypeScript + vitest — extension + unit tests
- `@vscode/vsce` — VSIX packaging, supports `--target`
- GitHub Actions — matrix build + release workflow

**Supported target matrix:**

| vsce target    | Rust triple                  | GitHub runner    |
|----------------|------------------------------|------------------|
| `darwin-arm64` | `aarch64-apple-darwin`       | `macos-14`       |
| `darwin-x64`   | `x86_64-apple-darwin`        | `macos-13`       |
| `linux-x64`    | `x86_64-unknown-linux-gnu`   | `ubuntu-latest`  |
| `linux-arm64`  | `aarch64-unknown-linux-gnu`  | `ubuntu-24.04-arm` |
| `win32-x64`    | `x86_64-pc-windows-msvc`     | `windows-latest` |

Native runners per target — no cargo-zigbuild, no cross-compilation complexity.

---

## File Structure

**New files:**
- `vscode/.vscodeignore` — excludes src/, tests/, node_modules dev deps, docs, coverage from VSIX
- `vscode/scripts/prepare-server.mjs` — Node script that copies the prebuilt `talkbank-lsp` binary into `vscode/server/` and sets the execute bit
- `vscode/src/test/executableService.bundled.test.ts` — unit tests for the new bundled-binary discovery path

**Modified files:**
- `vscode/src/executableService.ts` — add `findBundledBinary()`, call it before `findOnPath` in `findTalkbankLspBinary`; remove silent-failure default path
- `vscode/src/activation/lsp.ts` — surface a specific, actionable error when no binary is found (instead of letting spawn fail with ENOENT)
- `vscode/package.json` — add `package:<target>` and `package:all` scripts; drop `publish` until Marketplace is wired
- `vscode/DEVELOPER.md` — release procedure section
- `vscode/book/src/troubleshooting/lsp.md` — update discovery order
- `vscode/book/src/configuration/settings.md` — update `talkbank.lsp.binaryPath` discovery-order description
- `vscode/book/src/getting-started/installation.md` — update install instructions
- `.github/workflows/vscode-release.yml` — rewrite as matrix workflow (5 targets), attach VSIX files to a GitHub Release

**Deliberately NOT in scope (file separate follow-ups):**
- Marketplace publish step (`vsce publish --target ...`) — requires `VSCE_PAT` secret + publisher decision
- Bundling `send2clan` or `chatter` into the VSIX — separate tools, can follow same pattern later
- Code-signing / notarization for macOS binaries (see `docs/code-signing-and-distribution.md` in talkbank-dev)
- Auto-update mechanism — Marketplace handles this once enabled

---

## Phase 1: Discovery-Logic Refactor (TDD)

Goal: installed VSIX finds its bundled binary first; dev checkout still works; failures surface a helpful message instead of a spawn ENOENT.

### Task 1: RED — Bundled binary is preferred over dev-tree paths

**Files:**
- Test: `vscode/src/test/executableService.bundled.test.ts` (new)

- [ ] **Step 1: Write the failing test**

```typescript
import { describe, expect, it } from 'vitest';
import { ExecutableService } from '../executableService';
import { createMockExecSync, createMockFileSystem } from './mocks/childProcess';

describe('ExecutableService — bundled-binary discovery', () => {
    it('prefers the bundled extension/server/talkbank-lsp over everything else', () => {
        const mockFs = createMockFileSystem({
            '/extension/server/talkbank-lsp': 'binary',
            '/usr/local/bin/talkbank-lsp': 'also-binary',
            '/extension/../target/release/talkbank-lsp': 'dev-binary',
        });
        const execSync = createMockExecSync({
            stdout: '/usr/local/bin/talkbank-lsp\n',
        });
        const service = new ExecutableService({ fs: mockFs, execSync });
        const context = {
            asAbsolutePath: (rel: string) => `/extension/${rel}`,
        } as any;

        expect(service.findTalkbankLspBinary(context)).toBe(
            '/extension/server/talkbank-lsp',
        );
        // PATH lookup must not happen when bundled binary exists
        expect(execSync).not.toHaveBeenCalled();
    });

    it('uses server/talkbank-lsp.exe on Windows', () => {
        const mockFs = createMockFileSystem({
            '/extension/server/talkbank-lsp.exe': 'binary',
        });
        const execSync = createMockExecSync({ shouldThrow: true, errorMessage: 'which failed' });
        const service = new ExecutableService({ fs: mockFs, execSync });
        const context = {
            asAbsolutePath: (rel: string) => `/extension/${rel}`,
        } as any;
        const originalPlatform = process.platform;
        Object.defineProperty(process, 'platform', { value: 'win32' });
        try {
            expect(service.findTalkbankLspBinary(context)).toBe(
                '/extension/server/talkbank-lsp.exe',
            );
        } finally {
            Object.defineProperty(process, 'platform', { value: originalPlatform });
        }
    });

    it('falls through to PATH when no bundled binary', () => {
        const mockFs = createMockFileSystem({
            '/usr/local/bin/talkbank-lsp': 'binary',
        });
        const execSync = createMockExecSync({ stdout: '/usr/local/bin/talkbank-lsp\n' });
        const service = new ExecutableService({ fs: mockFs, execSync });
        const context = { asAbsolutePath: (rel: string) => `/extension/${rel}` } as any;

        expect(service.findTalkbankLspBinary(context)).toBe('/usr/local/bin/talkbank-lsp');
    });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd vscode && npx vitest run src/test/executableService.bundled.test.ts`

Expected: FAIL — the first test will receive `/usr/local/bin/talkbank-lsp` (current code checks PATH before bundled) or `/extension/../target/release/talkbank-lsp`. The second will land on the current silent-fallback path. The third should already pass.

### Task 2: GREEN — Add `findBundledBinary` and wire it first

**Files:**
- Modify: `vscode/src/executableService.ts:89-105` (replace `findTalkbankLspBinary`) and add private helper below `findExtensionTargetBinary`

- [ ] **Step 1: Add `findBundledBinary` helper**

After `findExtensionTargetBinary()` (around line 188), add:

```typescript
    private findBundledBinary(
        context: vscode.ExtensionContext,
        binaryName: string,
    ): string | null {
        const candidateName = process.platform === 'win32'
            ? `${binaryName}.exe`
            : binaryName;
        const candidate = context.asAbsolutePath(
            path.join('server', candidateName),
        );
        return this.fsOps.existsSync(candidate) ? candidate : null;
    }
```

- [ ] **Step 2: Change discovery order in `findTalkbankLspBinary`**

Replace the body (lines 89-105) with:

```typescript
    findTalkbankLspBinary(context: vscode.ExtensionContext, configuredPath = ''): string | null {
        if (configuredPath && this.fsOps.existsSync(configuredPath)) {
            return configuredPath;
        }

        const bundled = this.findBundledBinary(context, 'talkbank-lsp');
        if (bundled) {
            return bundled;
        }

        const pathBinary = this.findOnPath('talkbank-lsp');
        if (pathBinary && this.fsOps.existsSync(pathBinary)) {
            return pathBinary;
        }

        const localBuild = this.findExtensionTargetBinary(context, 'talkbank-lsp');
        if (localBuild) {
            return localBuild;
        }

        return null;
    }
```

Note the return type change: `string | null`. The old silent fallback (return a path that doesn't exist) caused the ENOENT that started this work.

- [ ] **Step 3: Update the interface signature**

In `vscode/src/executableService.ts` around line 38, change:

```typescript
    findTalkbankLspBinary(context: vscode.ExtensionContext, configuredPath?: string): string;
```

to:

```typescript
    findTalkbankLspBinary(context: vscode.ExtensionContext, configuredPath?: string): string | null;
```

- [ ] **Step 4: Run the new tests to verify they pass**

Run: `cd vscode && npx vitest run src/test/executableService.bundled.test.ts`

Expected: 3 passed.

- [ ] **Step 5: Run existing suite — catch any call-sites that now need null handling**

Run: `cd vscode && npm test`

Expected: compile errors or test failures at every call site that assumes a non-null return. The next task fixes those.

### Task 3: GREEN — Surface "binary not found" as a diagnostic, not an ENOENT

**Files:**
- Modify: `vscode/src/activation/lsp.ts:100` (call site of `findTalkbankLspBinary`)
- Modify: `vscode/src/utils/lspLocator.ts:16` (thin wrapper — propagates null)
- Test: extend `vscode/src/test/activationLsp.test.ts`

- [ ] **Step 1: Write the failing test for the null case**

Append to `vscode/src/test/activationLsp.test.ts`:

```typescript
it('shows an actionable error when no talkbank-lsp binary is found', async () => {
    const executableService = {
        findTalkbankLspBinary: vi.fn(() => null),
    };
    const showErrorMessage = vi.fn();
    const services = {
        executableService,
        // ...existing shims (copy from other tests in this file)
        window: { showErrorMessage },
    };

    await activateLsp(services as any, createTestContext());

    expect(showErrorMessage).toHaveBeenCalledWith(
        expect.stringContaining('talkbank-lsp'),
        'Open Settings',
    );
    // LSP client must NOT be started when there's no binary
    expect(services.languageClient?.start).not.toHaveBeenCalled();
});
```

(Adapt the test harness to match the file's existing patterns — if this test file mocks activation differently, mirror that style.)

- [ ] **Step 2: Run to verify it fails**

Run: `cd vscode && npx vitest run src/test/activationLsp.test.ts`

- [ ] **Step 3: Update `activateLsp` to handle null**

In `vscode/src/activation/lsp.ts` around line 100, wrap the binary lookup:

```typescript
const lspBinary = services.executableService.findTalkbankLspBinary(
    context,
    workspaceConfig.get<string>('lsp.binaryPath', ''),
);
if (!lspBinary) {
    const message =
        'TalkBank LSP binary not found. Install the platform-specific VSIX, ' +
        'place talkbank-lsp on PATH, or set "talkbank.lsp.binaryPath" to an absolute path.';
    const choice = await vscode.window.showErrorMessage(message, 'Open Settings');
    if (choice === 'Open Settings') {
        await vscode.commands.executeCommand(
            'workbench.action.openSettings',
            'talkbank.lsp.binaryPath',
        );
    }
    return;
}
```

- [ ] **Step 4: Verify tests pass**

Run: `cd vscode && npm test`

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
cd vscode
git add src/executableService.ts src/activation/lsp.ts src/test/executableService.bundled.test.ts src/test/activationLsp.test.ts src/utils/lspLocator.ts
git commit -m "feat(vscode): prefer bundled talkbank-lsp; surface actionable error on missing binary"
```

---

## Phase 2: VSIX Packaging

Goal: produce a small, per-platform VSIX that contains only what's needed to run, including the bundled binary.

### Task 4: Add `.vscodeignore`

**Files:**
- Create: `vscode/.vscodeignore`

- [ ] **Step 1: Write the ignore file**

```
# Build artifacts & dev-only files
src/**
tests/**
coverage/**
book/**
node_modules/@types/**
node_modules/typescript/**
node_modules/vitest/**
node_modules/@vitest/**
node_modules/eslint/**
node_modules/@typescript-eslint/**
node_modules/@vscode/test-electron/**
node_modules/@vscode/vsce/**
node_modules/fast-check/**

# TypeScript config
tsconfig.json
eslint.config.js
vitest.config.ts

# Developer docs (keep readme.md and images/, those are shown in Marketplace)
CLAUDE.md
DEVELOPER.md
CLAN-FEATURES.md
GUIDE.md
.gitignore
.vscodeignore

# Binary staging for OTHER platforms — only the matching one ships
# (vsce's --ignore-other-target-folders handles server.*/ folders; this line is a belt-and-suspenders)
scripts/**
```

- [ ] **Step 2: Verify the VSIX is much smaller**

Run: `cd vscode && npx vsce package --out /tmp/test.vsix`
Run: `ls -lh /tmp/test.vsix && unzip -l /tmp/test.vsix | wc -l`

Expected: VSIX dropped from 35 MB / 4852 files to a few MB / low-hundreds-of-files. Record the numbers in the commit message.

- [ ] **Step 3: Commit**

```bash
cd vscode
git add .vscodeignore
git commit -m "build(vscode): add .vscodeignore — strip dev deps and source from VSIX"
```

### Task 5: Binary-staging script

**Files:**
- Create: `vscode/scripts/prepare-server.mjs`
- Create: `vscode/server/.gitignore` — contents: `*` followed by `!.gitignore`, so the directory exists in git but staged binaries don't
- Modify: `vscode/package.json` — add scripts

- [ ] **Step 1: Write the staging script**

`vscode/scripts/prepare-server.mjs`:

```javascript
#!/usr/bin/env node
// Copies a prebuilt talkbank-lsp binary into vscode/server/ before packaging.
// Usage: node scripts/prepare-server.mjs --source <path> [--windows]
import { copyFileSync, chmodSync, mkdirSync, existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

const { values } = parseArgs({
    options: {
        source: { type: 'string' },
        windows: { type: 'boolean', default: false },
    },
});

if (!values.source) {
    console.error('error: --source <path-to-talkbank-lsp> is required');
    process.exit(1);
}

const source = resolve(values.source);
if (!existsSync(source)) {
    console.error(`error: source binary does not exist: ${source}`);
    process.exit(1);
}

const extensionRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const serverDir = resolve(extensionRoot, 'server');
const destName = values.windows ? 'talkbank-lsp.exe' : 'talkbank-lsp';
const dest = resolve(serverDir, destName);

mkdirSync(serverDir, { recursive: true });
copyFileSync(source, dest);
if (!values.windows) {
    chmodSync(dest, 0o755);
}

console.log(`staged ${source} -> ${dest}`);
```

- [ ] **Step 2: Keep `vscode/server/` in git but don't track binaries**

```bash
mkdir -p vscode/server
printf '*\n!.gitignore\n' > vscode/server/.gitignore
```

- [ ] **Step 3: Add per-target npm scripts**

In `vscode/package.json`, under `"scripts"`, add:

```json
"package:prepare": "node scripts/prepare-server.mjs",
"package:darwin-arm64": "vsce package --target darwin-arm64 --out talkbank-chat-darwin-arm64.vsix",
"package:darwin-x64":   "vsce package --target darwin-x64   --out talkbank-chat-darwin-x64.vsix",
"package:linux-x64":    "vsce package --target linux-x64    --out talkbank-chat-linux-x64.vsix",
"package:linux-arm64":  "vsce package --target linux-arm64  --out talkbank-chat-linux-arm64.vsix",
"package:win32-x64":    "vsce package --target win32-x64    --out talkbank-chat-win32-x64.vsix"
```

(Keep the existing `"package": "vsce package"` for developer convenience — produces a universal VSIX without bundled binary.)

- [ ] **Step 4: Smoke-test locally end-to-end on darwin-arm64**

```bash
cd /Users/chen/talkbank/talkbank-tools
cargo build --release -p talkbank-lsp
cd vscode
node scripts/prepare-server.mjs --source ../target/release/talkbank-lsp
npm run package:darwin-arm64
unzip -l talkbank-chat-darwin-arm64.vsix | grep server/
```

Expected: `extension/server/talkbank-lsp` listed in the VSIX with size matching the release binary (~20-30 MB).

- [ ] **Step 5: Install the VSIX cold and verify it activates**

```bash
# Uninstall the current broken extension
code-insiders --uninstall-extension talkbank.talkbank-chat
code-insiders --install-extension vscode/talkbank-chat-darwin-arm64.vsix
# Open a .cha file and confirm LSP activates — no "spawn ENOENT"
code-insiders /tmp/example.cha  # any .cha file
```

Expected: LSP activates cleanly; `talkbank.lsp.binaryPath` setting is empty; diagnostics appear in the editor.

- [ ] **Step 6: Commit**

```bash
cd vscode
git add scripts/ server/.gitignore package.json package-lock.json
git commit -m "build(vscode): stage talkbank-lsp into server/ and add per-target vsce scripts"
```

---

## Phase 3: CI Release Workflow

Goal: replace the single-target `vscode-release.yml` with a matrix build that produces 5 VSIX files and attaches them to a GitHub Release.

### Task 6: Rewrite `vscode-release.yml` as a matrix build

**Files:**
- Modify: `.github/workflows/vscode-release.yml`

- [ ] **Step 1: Replace the workflow**

```yaml
name: VS Code Extension Release

on:
  workflow_dispatch:
    inputs:
      version:
        description: "Extension version to release (must match package.json)"
        required: true

jobs:
  verify:
    name: Verify metadata
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: actions/setup-node@v6
        with:
          node-version: '24'
          cache: npm
          cache-dependency-path: vscode/package-lock.json
      - working-directory: vscode
        run: |
          npm ci
          npm run compile
          npm run lint
          npm run test
      - name: Version / license / publisher check
        working-directory: vscode
        run: |
          PKG_VERSION=$(node -e "console.log(require('./package.json').version)")
          PKG_LICENSE=$(node -e "console.log(require('./package.json').license)")
          PKG_PUBLISHER=$(node -e "console.log(require('./package.json').publisher)")
          test "$PKG_VERSION" = "${{ inputs.version }}" || { echo "::error::version mismatch"; exit 1; }
          test "$PKG_LICENSE" = "BSD-3-Clause" || { echo "::error::bad license"; exit 1; }
          test -n "$PKG_PUBLISHER" || { echo "::error::no publisher"; exit 1; }

  build:
    needs: verify
    name: Build ${{ matrix.target }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: darwin-arm64
            runner: macos-14
            rust-target: aarch64-apple-darwin
            exe-suffix: ""
            prepare-flags: ""
          - target: darwin-x64
            runner: macos-13
            rust-target: x86_64-apple-darwin
            exe-suffix: ""
            prepare-flags: ""
          - target: linux-x64
            runner: ubuntu-latest
            rust-target: x86_64-unknown-linux-gnu
            exe-suffix: ""
            prepare-flags: ""
          - target: linux-arm64
            runner: ubuntu-24.04-arm
            rust-target: aarch64-unknown-linux-gnu
            exe-suffix: ""
            prepare-flags: ""
          - target: win32-x64
            runner: windows-latest
            rust-target: x86_64-pc-windows-msvc
            exe-suffix: ".exe"
            prepare-flags: "--windows"
    runs-on: ${{ matrix.runner }}
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.rust-target }}
      - uses: Swatinem/rust-cache@v2
        with:
          key: vsix-${{ matrix.target }}
      - uses: actions/setup-node@v6
        with:
          node-version: '24'
          cache: npm
          cache-dependency-path: vscode/package-lock.json

      - name: Build talkbank-lsp
        run: cargo build --release -p talkbank-lsp --target ${{ matrix.rust-target }}

      - name: Install extension deps
        working-directory: vscode
        run: npm ci

      - name: Compile extension
        working-directory: vscode
        run: npm run compile

      - name: Stage server binary
        working-directory: vscode
        shell: bash
        run: |
          node scripts/prepare-server.mjs \
            --source ../target/${{ matrix.rust-target }}/release/talkbank-lsp${{ matrix.exe-suffix }} \
            ${{ matrix.prepare-flags }}

      - name: Package VSIX
        working-directory: vscode
        run: npm run package:${{ matrix.target }}

      - name: Upload VSIX artifact
        uses: actions/upload-artifact@v6
        with:
          name: talkbank-chat-${{ matrix.target }}-${{ inputs.version }}
          path: vscode/talkbank-chat-${{ matrix.target }}.vsix
          if-no-files-found: error

  release:
    needs: build
    name: Create GitHub Release
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v6
      - uses: actions/download-artifact@v6
        with:
          path: artifacts
          merge-multiple: true
      - name: Create release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: vscode-v${{ inputs.version }}
          name: VS Code Extension v${{ inputs.version }}
          generate_release_notes: true
          files: artifacts/*.vsix
          fail_on_unmatched_files: true
```

- [ ] **Step 2: Verify the workflow YAML parses**

Run: `cd /Users/chen/talkbank/talkbank-tools && gh workflow view vscode-release.yml --yaml || true`

(Before pushing, a local check: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/vscode-release.yml'))"`)

- [ ] **Step 3: Commit**

```bash
cd /Users/chen/talkbank/talkbank-tools
git add .github/workflows/vscode-release.yml
git commit -m "ci(vscode): matrix release — 5 platforms, bundled talkbank-lsp, GitHub Release"
```

- [ ] **Step 4: Pause for Franklin's approval before triggering the workflow**

Before running the workflow, confirm with Franklin:
- Is version `0.1.0` the correct release target, or should this bump to `0.2.0` to mark the bundled-binary epoch?
- Does the version string need updating in `package.json` first?

Do not `gh workflow run` without explicit approval (per CLAUDE.md "Never push without explicit approval" rule, which covers anything that creates shared state like a GitHub Release).

---

## Phase 4: Documentation & Verification

### Task 7: Update extension docs to describe bundled-binary reality

**Files:**
- Modify: `vscode/DEVELOPER.md` — new "Releasing" section
- Modify: `vscode/book/src/troubleshooting/lsp.md` — discovery order
- Modify: `vscode/book/src/configuration/settings.md` — binaryPath description
- Modify: `vscode/book/src/getting-started/installation.md` — install from GitHub Release

- [ ] **Step 1: Update discovery-order descriptions in all three book pages**

Replace every occurrence of the current 3-step discovery description with:

```
1. **`talkbank.lsp.binaryPath` setting** (if set and the file exists) — explicit override
2. **Bundled binary** at `<extension>/server/talkbank-lsp[.exe]` — shipped with platform-specific VSIX
3. **System PATH** via `which talkbank-lsp` — for developers who install via `cargo install`
4. **Dev-tree builds** at `<extension>/../target/{debug,release}/talkbank-lsp` — for contributors running from a cloned workspace
5. If none found, an error notification appears with a link to the setting.
```

- [ ] **Step 2: Add a Releasing section to `vscode/DEVELOPER.md`**

After the existing architecture content, append:

```markdown
## Releasing

The extension is published as five platform-specific VSIX files via the
`VS Code Extension Release` workflow (`.github/workflows/vscode-release.yml`).
Each VSIX bundles a prebuilt `talkbank-lsp` for its target platform at
`extension/server/talkbank-lsp[.exe]`.

### Cutting a release

1. Bump `vscode/package.json` version.
2. Commit and push to `main`.
3. Trigger the workflow: `gh workflow run vscode-release.yml -f version=X.Y.Z`
4. Wait for all five matrix jobs (darwin-arm64, darwin-x64, linux-x64,
   linux-arm64, win32-x64) to succeed.
5. The workflow creates a GitHub Release tagged `vscode-vX.Y.Z` with the
   five VSIX files attached.

### Local packaging (single platform)

```bash
cd /Users/chen/talkbank/talkbank-tools
cargo build --release -p talkbank-lsp
cd vscode
node scripts/prepare-server.mjs --source ../target/release/talkbank-lsp
npm run package:darwin-arm64   # or whichever target matches your host
```

### Users installing the VSIX

1. Go to https://github.com/TalkBank/talkbank-tools/releases/latest
2. Download the VSIX matching your platform.
3. `code --install-extension talkbank-chat-<platform>.vsix`

No Rust toolchain or PATH setup required.
```

- [ ] **Step 3: Stamp "Last modified" on every touched doc**

Run `date '+%Y-%m-%d %H:%M %Z'` and update the `Last modified` header in each file touched in this task.

- [ ] **Step 4: Commit**

```bash
cd /Users/chen/talkbank/talkbank-tools
git add vscode/DEVELOPER.md vscode/book/src/troubleshooting/lsp.md vscode/book/src/configuration/settings.md vscode/book/src/getting-started/installation.md
git commit -m "docs(vscode): describe bundled-binary discovery order and release procedure"
```

### Task 8: End-to-end verification on a clean VS Code profile

**Files:** none (verification only)

- [ ] **Step 1: Verify from a clean profile on your own machine**

```bash
# Create a throwaway profile — avoids polluting the main config
code-insiders --user-data-dir=/tmp/vsc-verify --extensions-dir=/tmp/vsc-verify-ext &
sleep 3
code-insiders --user-data-dir=/tmp/vsc-verify --extensions-dir=/tmp/vsc-verify-ext \
    --install-extension /Users/chen/talkbank/talkbank-tools/vscode/talkbank-chat-darwin-arm64.vsix
# Open a CHAT file
code-insiders --user-data-dir=/tmp/vsc-verify --extensions-dir=/tmp/vsc-verify-ext \
    /Users/chen/talkbank/talkbank-tools/corpus/reference/any-real-file.cha
```

Verify in the throwaway VS Code window:
- The TalkBank Language Server Output channel shows startup without ENOENT
- Diagnostics appear in the gutter
- `Cmd+Shift+G` (dependency graph) works on an utterance
- Cleanup: `rm -rf /tmp/vsc-verify /tmp/vsc-verify-ext`

- [ ] **Step 2: Verify the setting-based override still works**

In the clean profile, add `"talkbank.lsp.binaryPath": "/tmp/nonexistent"` to settings and reload. The error notification must appear with `Open Settings` action.

- [ ] **Step 3: Verify dev-tree fallback still works for contributors**

Delete the installed extension, then from a clone:

```bash
cd /Users/chen/talkbank/talkbank-tools/vscode
npm run compile
# Use F5 "Run Extension" in VS Code to launch the extension in a host window.
# Open a .cha file. Expect LSP to activate from ../target/debug/talkbank-lsp.
```

- [ ] **Step 4: Report results back to Franklin with a one-line summary per check.**

---

## Self-Review

1. **Spec coverage:** every stated goal is covered — bundled binary (Phase 2), cleaner error (Task 3), CI matrix (Phase 3), docs (Task 7), smoke verification (Task 8).
2. **Placeholder scan:** no TBDs, no "add appropriate error handling", every code block is concrete.
3. **Type consistency:** `findTalkbankLspBinary` return type flipped from `string` to `string | null` in Task 2, and Task 3 updates the call site to match. The interface declaration (line 38 of `executableService.ts`) is updated in Task 2 Step 3.
4. **Cross-task dependencies:**
   - Task 5 Step 5's local smoke test depends on Task 2's GREEN state.
   - Task 6's workflow depends on Task 5's npm scripts existing.
   - Task 7 Step 2 documents behavior introduced in Tasks 1-3 and 6.

---

## Execution Handoff

**Which approach, Franklin?**

1. **Subagent-Driven** (recommended) — I dispatch a fresh subagent per task, review between tasks. Good for 8 tasks of distinct scope.
2. **Inline Execution** — I execute tasks in this session with checkpoints at each phase boundary.

Either way, the Phase 3 workflow push and the Phase 4 Step 4 "trigger workflow" step will pause for your approval per the never-push-without-approval rule.
