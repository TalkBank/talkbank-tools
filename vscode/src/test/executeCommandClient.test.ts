/**
 * Tests for the Effect-native execute-command client facade.
 */

/* eslint-disable @typescript-eslint/naming-convention */

import { Effect } from 'effect';
import { describe, expect, it, vi } from 'vitest';

import {
    ExecuteCommandRequestError,
    ExecuteCommandResponseError,
    TalkbankExecuteCommandClient,
} from '../lsp/executeCommandClient';

function createClient(sendRequest: (method: string, payload: unknown) => Promise<unknown>) {
    return new TalkbankExecuteCommandClient({
        isRunning: () => true,
        sendRequest,
    } as never);
}

describe('execute command client', () => {
    it('returns typed string results through Effect', async () => {
        const client = createClient(async () => 'digraph G {}');

        await expect(Effect.runPromise(client.showDependencyGraph(
            'file:///tmp/sample.cha',
            { line: 4, character: 2 } as never,
        ))).resolves.toBe('digraph G {}');
    });

    it('wraps transport failures in a tagged request error', async () => {
        const client = createClient(async () => {
            throw new Error('socket closed');
        });

        const result = await Effect.runPromise(Effect.either(client.getAlignmentSidecar(
            'file:///tmp/sample.cha',
        )));
        expect(result._tag).toBe('Left');
        if (result._tag === 'Left') {
            expect(result.left).toBeInstanceOf(ExecuteCommandRequestError);
            expect(result.left.command).toBe('talkbank/getAlignmentSidecar');
        }
    });

    it('wraps unexpected string commands responses in a tagged response error', async () => {
        const client = createClient(async () => ({ ok: false }));

        const result = await Effect.runPromise(Effect.either(client.formatIdLine({
            language: 'eng',
            corpus: 'Example',
            speaker: 'CHI',
            age: '2;03',
            sex: 'male',
            group: '',
            ses: '',
            role: 'Target_Child',
            education: '',
            custom: '',
        })));
        expect(result._tag).toBe('Left');
        if (result._tag === 'Left') {
            expect(result.left).toBeInstanceOf(ExecuteCommandResponseError);
            expect(result.left.command).toBe('talkbank/formatIdLine');
        }
    });
});
