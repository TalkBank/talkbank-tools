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

    it('prefers a configured chatter path for LSP activation when it exists', () => {
        const mockFs = createMockFileSystem({
            '/tmp/chatter': 'binary'
        });
        const execSync = createMockExecSync({
            shouldThrow: true,
            errorMessage: 'Command failed: which chatter'
        });
        const service = new ExecutableService({
            fs: mockFs,
            execSync
        });
        const context = {
            asAbsolutePath: (relativePath: string) => `/extension/${relativePath}`
        } as any;

        expect(service.findTalkbankLspBinary(context, '/tmp/chatter')).toBe('/tmp/chatter');
        expect(execSync).not.toHaveBeenCalled();
    });

    it('finds chatter on PATH for LSP activation', () => {
        const mockFs = createMockFileSystem({
            '/usr/local/bin/chatter': 'binary'
        });
        const execSync = createMockExecSync({
            stdout: '/usr/local/bin/chatter\n'
        });
        const service = new ExecutableService({
            fs: mockFs,
            execSync
        });
        const context = {
            asAbsolutePath: (relativePath: string) => `/extension/${relativePath}`
        } as any;

        expect(service.findTalkbankLspBinary(context)).toBe('/usr/local/bin/chatter');
        expect(execSync).toHaveBeenCalledWith(
            'which chatter',
            expect.objectContaining({
                encoding: 'utf-8',
                stdio: ['pipe', 'pipe', 'pipe']
            })
        );
    });

    it('falls back to local chatter builds for LSP activation', () => {
        const mockFs = createMockFileSystem({
            '/extension/../target/release/chatter': 'binary'
        });
        const execSync = createMockExecSync({
            shouldThrow: true,
            errorMessage: 'Command failed: which chatter'
        });
        const service = new ExecutableService({
            fs: mockFs,
            execSync
        });
        const context = {
            asAbsolutePath: (relativePath: string) => `/extension/${relativePath}`
        } as any;

        expect(service.findTalkbankLspBinary(context)).toBe('/extension/../target/release/chatter');
    });

    it('uses chatter.exe for the final Windows fallback path', () => {
        const execSync = createMockExecSync({
            shouldThrow: true,
            errorMessage: 'Command failed: which chatter'
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
            expect(service.findTalkbankLspBinary(context)).toBe('/extension/../target/debug/chatter.exe');
        } finally {
            Object.defineProperty(process, 'platform', { value: originalPlatform });
        }
    });
});
