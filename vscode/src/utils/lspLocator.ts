/**
 * Utility for locating the `talkbank-lsp` binary used for TalkBank LSP activation.
 */

import type * as vscode from 'vscode';

import type { ExtensionRuntimeServices } from '../runtimeServices';

/**
 * Find the standalone `talkbank-lsp` binary that the VS Code extension spawns
 * as its language server. Ships alongside `chatter` in the release archive.
 *
 * @param context - Extension activation context.
 * @returns Absolute path to the LSP server binary.
 */
export function findTalkbankLspBinary(
    context: vscode.ExtensionContext,
    services: Pick<ExtensionRuntimeServices, 'executableService' | 'runtimeContext'>,
): string {
    return services.executableService.findTalkbankLspBinary(
        context,
        services.runtimeContext.getConfiguredLspBinaryPath(),
    );
}
