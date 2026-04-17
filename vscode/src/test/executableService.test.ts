import { describe, expect, it, vi } from 'vitest';

import { ExecutableService } from '../executableService';
import { createMockExecSync, createMockFileSystem } from './mocks/childProcess';

describe('ExecutableService', () => {
    it('runs talkbank CLI commands through one shared boundary', () => {
        const execSync = createMockExecSync({
            stdout: 'ok'
        });
        const service = new ExecutableService({ execSync });

        const result = service.runTalkBankCli('/tmp/talkbank', [
            'validate',
            '/tmp/example.cha',
            '--json'
        ]);

        expect(result).toBe('ok');
        expect(execSync).toHaveBeenCalledWith(
            '"/tmp/talkbank" validate "/tmp/example.cha" --json',
            expect.objectContaining({
                encoding: 'utf-8',
                stdio: ['pipe', 'pipe', 'pipe']
            })
        );
    });

    it('finds send2clan by walking up to target build directories', async () => {
        const mockFs = createMockFileSystem({
            '/workspace/target/debug/send2clan': 'binary'
        });
        const execSync = createMockExecSync({
            shouldThrow: true,
            errorMessage: 'Command failed: which send2clan'
        });
        const service = new ExecutableService({
            fs: mockFs,
            execSync
        });

        await expect(
            service.findSend2ClanBinary('/workspace/project/subdir')
        ).resolves.toBe('/workspace/target/debug/send2clan');
    });

    it('spawns detached background processes through one helper', () => {
        const unref = vi.fn();
        const spawnFn = vi.fn(() => ({ unref }) as any);
        const service = new ExecutableService({ spawn: spawnFn });

        service.spawnDetached('/tmp/send2clan', ['/tmp/file.cha', '10', '2']);

        expect(spawnFn).toHaveBeenCalledWith(
            '/tmp/send2clan',
            ['/tmp/file.cha', '10', '2'],
            {
                detached: true,
                stdio: 'ignore'
            }
        );
        expect(unref).toHaveBeenCalled();
    });

    it('prefers a configured talkbank-lsp path for LSP activation when it exists', () => {
        const mockFs = createMockFileSystem({
            '/tmp/talkbank-lsp': 'binary'
        });
        const execSync = createMockExecSync({
            shouldThrow: true,
            errorMessage: 'Command failed: which talkbank-lsp'
        });
        const service = new ExecutableService({
            fs: mockFs,
            execSync
        });
        const context = {
            asAbsolutePath: (relativePath: string) => `/extension/${relativePath}`
        } as any;

        expect(service.findTalkbankLspBinary(context, '/tmp/talkbank-lsp')).toBe('/tmp/talkbank-lsp');
        expect(execSync).not.toHaveBeenCalled();
    });

    it('finds talkbank-lsp on PATH for LSP activation', () => {
        const mockFs = createMockFileSystem({
            '/usr/local/bin/talkbank-lsp': 'binary'
        });
        const execSync = createMockExecSync({
            stdout: '/usr/local/bin/talkbank-lsp\n'
        });
        const service = new ExecutableService({
            fs: mockFs,
            execSync
        });
        const context = {
            asAbsolutePath: (relativePath: string) => `/extension/${relativePath}`
        } as any;

        expect(service.findTalkbankLspBinary(context)).toBe('/usr/local/bin/talkbank-lsp');
        expect(execSync).toHaveBeenCalledWith(
            'which talkbank-lsp',
            expect.objectContaining({
                encoding: 'utf-8',
                stdio: ['pipe', 'pipe', 'pipe']
            })
        );
    });

    it('falls back to local talkbank-lsp builds for LSP activation', () => {
        const mockFs = createMockFileSystem({
            '/extension/../target/release/talkbank-lsp': 'binary'
        });
        const execSync = createMockExecSync({
            shouldThrow: true,
            errorMessage: 'Command failed: which talkbank-lsp'
        });
        const service = new ExecutableService({
            fs: mockFs,
            execSync
        });
        const context = {
            asAbsolutePath: (relativePath: string) => `/extension/${relativePath}`
        } as any;

        expect(service.findTalkbankLspBinary(context)).toBe('/extension/../target/release/talkbank-lsp');
    });

    it('returns null on Windows when no binary can be discovered', () => {
        const execSync = createMockExecSync({
            shouldThrow: true,
            errorMessage: 'Command failed: which talkbank-lsp'
        });
        const service = new ExecutableService({
            fs: createMockFileSystem({}),
            execSync
        });
        const context = {
            asAbsolutePath: (relativePath: string) => `/extension/${relativePath}`
        } as any;
        const originalPlatform = process.platform;
        Object.defineProperty(process, 'platform', { value: 'win32' });

        try {
            expect(service.findTalkbankLspBinary(context)).toBeNull();
        } finally {
            Object.defineProperty(process, 'platform', { value: originalPlatform });
        }
    });
});
