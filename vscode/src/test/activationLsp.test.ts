import { describe, expect, it, vi } from 'vitest';

vi.mock('vscode', () => ({
    DiagnosticSeverity: {
        Error: 0,
        Warning: 1,
        Information: 2,
    },
}));

vi.mock('vscode-languageclient/node', () => ({
    LanguageClient: class {},
    TransportKind: {
        stdio: 0,
    },
}));

import {
    activateLanguageServer,
    createLanguageClientOptions,
} from '../activation/lsp';
import type { ExtensionExecutableService } from '../runtimeServices';
import type { ExtensionRuntimeContext, ValidationSeverity } from '../runtimeContext';

function createRuntimeContext(overrides: Partial<ExtensionRuntimeContext> = {}): ExtensionRuntimeContext {
    return {
        getPrimaryWorkspaceRoot: () => '/workspace',
        getConfiguredLspBinaryPath: () => '',
        getInlayHintsEnabled: () => true,
        getValidationSeverity: () => 'all',
        getBulletDisplayMode: () => 'dim',
        getWalkerAutoPlay: () => true,
        getWalkerLoopCount: () => 1,
        getWalkerPauseSeconds: () => 0,
        getWalkerWalkLength: () => 0,
        getMediaDefaultSpeed: () => 100,
        getTranscriptionDefaultSpeaker: () => 'CHI',
        getTranscriptionRewindSeconds: () => 2,
        getCachePath: () => '/cache/talkbank-cache.db',
        ...overrides,
    };
}

function createExecutableService(
    overrides: Partial<ExtensionExecutableService> = {},
): ExtensionExecutableService {
    return {
        findTalkBankCli: vi.fn(async () => null),
        runTalkBankCli: vi.fn(() => ''),
        findSend2ClanBinary: vi.fn(async () => null),
        spawnDetached: vi.fn(),
        findTalkbankLspBinary: vi.fn(() => '/tmp/chatter'),
        ...overrides,
    };
}

describe('LSP activation', () => {
    it('builds middleware from the injected runtime context', () => {
        let severity: ValidationSeverity = 'errorsOnly';
        let inlayHintsEnabled = true;
        const nextDiagnostics = vi.fn();
        const nextInlayHints = vi.fn(() => ['hint']);
        const uri = { path: '/workspace/sample.cha' } as never;
        const diagnostics = [
            { severity: 0, label: 'error' },
            { severity: 1, label: 'warning' },
            { severity: 2, label: 'info' },
        ];

        const options = createLanguageClientOptions({
            workspace: {
                createFileSystemWatcher: vi.fn(() => 'watcher' as never),
            },
            runtimeContext: createRuntimeContext({
                getValidationSeverity: () => severity,
                getInlayHintsEnabled: () => inlayHintsEnabled,
            }),
        });

        options.middleware?.handleDiagnostics?.(uri, diagnostics as never, nextDiagnostics);
        expect(nextDiagnostics).toHaveBeenLastCalledWith(uri, [diagnostics[0]]);

        severity = 'errorsAndWarnings';
        options.middleware?.handleDiagnostics?.(uri, diagnostics as never, nextDiagnostics);
        expect(nextDiagnostics).toHaveBeenLastCalledWith(uri, [diagnostics[0], diagnostics[1]]);

        inlayHintsEnabled = false;
        expect(options.middleware?.provideInlayHints?.(
            {} as never,
            {} as never,
            {} as never,
            nextInlayHints as never,
        )).toEqual([]);
        expect(nextInlayHints).not.toHaveBeenCalled();
    });

    it('activates the language client through injected services', () => {
        const createLanguageClient = vi.fn(() => ({
            start: vi.fn(),
        } as never));
        const executableService = createExecutableService({
            findTalkbankLspBinary: vi.fn(() => '/tmp/custom-chatter'),
        });
        const workspace = {
            createFileSystemWatcher: vi.fn(() => 'watcher' as never),
        };
        const runtimeContext = createRuntimeContext({
            getConfiguredLspBinaryPath: () => '/tmp/configured-chatter',
        });
        const context = {
            asAbsolutePath: (relativePath: string) => `/extension/${relativePath}`,
        } as never;

        const client = activateLanguageServer(context, {
            executableService,
            runtimeContext,
            workspace,
            createLanguageClient,
        });

        expect(executableService.findTalkbankLspBinary).toHaveBeenCalledWith(
            context,
            '/tmp/configured-chatter',
        );
        expect(createLanguageClient).toHaveBeenCalledWith(
            'talkbank-lsp',
            'TalkBank Language Server',
            expect.objectContaining({
                run: expect.objectContaining({
                    command: '/tmp/custom-chatter',
                    args: ['lsp'],
                }),
                debug: expect.objectContaining({
                    command: '/tmp/custom-chatter',
                    args: ['lsp'],
                }),
            }),
            expect.objectContaining({
                synchronize: {
                    fileEvents: 'watcher',
                },
            }),
        );
        expect((client as any).start).toHaveBeenCalled();
    });
});
