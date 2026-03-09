/**
 * Utility for locating the `chatter` binary used for TalkBank LSP activation.
 */

import type * as vscode from 'vscode';

import type { ExtensionRuntimeServices } from '../runtimeServices';

/**
 * Find the `chatter` binary used to launch `chatter lsp` for the VS Code extension.
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
