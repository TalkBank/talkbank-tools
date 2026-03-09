/**
 * Command handler for dependency-graph generation.
 */

import { Effect } from 'effect';

import { GraphPanel } from '../graphPanel';
import {
    EmptyCommandResponseError,
    ExtensionCommandRequirements,
    requireActiveEditor,
} from '../effectCommandRuntime';
import {
    ExecuteCommandClientService,
    ExtensionContextService,
} from '../effectRuntime';

/**
 * Generate and show the dependency graph for the current cursor position.
 */
export function showDependencyGraph(): Effect.Effect<
    void,
    unknown,
    ExtensionCommandRequirements
> {
    return Effect.gen(function*() {
        const editor = yield* requireActiveEditor();
        const commands = yield* ExecuteCommandClientService;
        const context = yield* ExtensionContextService;
        const document = editor.document;
        const position = editor.selection.active;

        const dotOutput = yield* commands.showDependencyGraph(
            document.uri.toString(),
            position,
        );
        if (!dotOutput) {
            return yield* Effect.fail(new EmptyCommandResponseError({
                command: 'talkbank/showDependencyGraph',
            }));
        }

        yield* Effect.sync(() => {
            GraphPanel.createOrShow(context, dotOutput, document.fileName);
        });
    });
}
