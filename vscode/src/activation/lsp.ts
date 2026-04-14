/**
 * Language-server activation helpers for the TalkBank VS Code extension.
 */

import * as vscode from 'vscode';
import {
    LanguageClient,
    TransportKind
} from 'vscode-languageclient/node';
import type {
    LanguageClientOptions,
    ServerOptions,
} from 'vscode-languageclient/node';
import {
    type ValidationSeverity,
} from '../runtimeContext';
import type { ExtensionRuntimeServices } from '../runtimeServices';

export interface LanguageServerWorkspace {
    createFileSystemWatcher(globPattern: string): vscode.FileSystemWatcher;
}

export type LanguageClientFactory = (
    id: string,
    name: string,
    serverOptions: ServerOptions,
    clientOptions: LanguageClientOptions,
) => LanguageClient;

export interface LanguageServerActivationServices extends ExtensionRuntimeServices {
    readonly workspace: LanguageServerWorkspace;
    readonly createLanguageClient?: LanguageClientFactory;
}

/**
 * Filter diagnostics according to the current runtime validation-severity setting.
 */
export function filterDiagnosticsBySeverity<T extends { severity?: vscode.DiagnosticSeverity }>(
    diagnostics: readonly T[],
    severity: ValidationSeverity,
): T[] {
    if (severity === 'errorsOnly') {
        return diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Error);
    }

    if (severity === 'errorsAndWarnings') {
        return diagnostics.filter(d =>
            d.severity === vscode.DiagnosticSeverity.Error ||
            d.severity === vscode.DiagnosticSeverity.Warning
        );
    }

    return [...diagnostics];
}

/**
 * Build language-client options from injected runtime services.
 */
export function createLanguageClientOptions(
    services: Pick<LanguageServerActivationServices, 'workspace' | 'runtimeContext'>,
): LanguageClientOptions {
    const { workspace, runtimeContext } = services;

    return {
        documentSelector: [{ scheme: 'file', language: 'chat' }],
        synchronize: {
            fileEvents: workspace.createFileSystemWatcher('**/*.cha')
        },
        initializationOptions: {
            inlayHintsEnabled: runtimeContext.getInlayHintsEnabled(),
        },
        middleware: {
            handleDiagnostics(uri, diagnostics, next) {
                next(uri, filterDiagnosticsBySeverity(
                    diagnostics,
                    runtimeContext.getValidationSeverity(),
                ));
            },
            provideInlayHints(document, range, token, next) {
                if (!runtimeContext.getInlayHintsEnabled()) {
                    return [];
                }

                return next(document, range, token);
            },
        },
    };
}

/**
 * Start and return the TalkBank language client.
 *
 * @param context - Extension activation context.
 * @returns Started language client instance.
 */
export function activateLanguageServer(
    context: vscode.ExtensionContext,
    services: LanguageServerActivationServices,
): LanguageClient {
    const lspBinary = services.executableService.findTalkbankLspBinary(
        context,
        services.runtimeContext.getConfiguredLspBinaryPath(),
    );
    // The standalone `talkbank-lsp` binary takes no subcommand — it speaks LSP
    // over stdio as soon as it starts.
    const serverArgs: string[] = [];
    const debugOptions = { execArgv: ['--nolazy', '--inspect=6009'] };

    const serverOptions: ServerOptions = {
        run: { command: lspBinary, args: serverArgs, transport: TransportKind.stdio } as any,
        debug: {
            command: lspBinary,
            args: serverArgs,
            transport: TransportKind.stdio,
            options: debugOptions
        } as any
    };

    const clientOptions = createLanguageClientOptions(services);
    const createLanguageClient =
        services.createLanguageClient ??
        ((id, name, server, client) => new LanguageClient(id, name, server, client));
    const client = createLanguageClient(
        'talkbank-lsp',
        'TalkBank Language Server',
        serverOptions,
        clientOptions,
    );
    client.start();
    return client;
}
