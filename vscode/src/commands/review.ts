/**
 * Review mode commands for alignment quality evaluation.
 *
 * Workflow:
 * 1. User opens a CHAT file with `%xrev` tiers (from `batchalign3 align --bullet-repair`)
 * 2. `startReview` parses all `%xrev` items, activates review state
 * 3. User navigates between flagged utterances with `reviewNext`/`reviewPrev`
 * 4. User rates each utterance: good/early/late/wrong/skip
 * 5. Rating edits the `%xrev:` line in the document buffer
 * 6. Auto-advance to next unreviewed utterance
 *
 * No webview panel needed — ratings edit the document directly, decorations
 * provide visual feedback, and the status bar shows progress.
 */

import * as vscode from 'vscode';
import { Effect } from 'effect';

import {
    ReviewCommandStateService,
    type ReviewCommandState,
} from '../reviewState';
import {
    parseReviewItems,
    countReviewed,
    nextUnreviewed,
    prevUnreviewed,
    formatXrevContent,
    type ReviewStatus,
} from '../reviewModel';
import {
    type ExtensionCommandRequirements,
    VSCodeCommandsService,
    VSCodeWindowService,
    requireActiveChatEditor,
} from '../effectCommandRuntime';
import { tryAsync } from '../effectRuntime';

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

let statusBarItem: vscode.StatusBarItem | undefined;

function updateStatusBar(items: readonly { status: string }[]): void {
    if (!statusBarItem) {
        statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Left,
            50,
        );
    }
    const total = items.length;
    const reviewed = items.filter(i => i.status !== '?').length;
    statusBarItem.text = `$(checklist) Review: ${reviewed}/${total}`;
    statusBarItem.tooltip = `Alignment review: ${reviewed} of ${total} utterances rated`;
    statusBarItem.show();
}

function hideStatusBar(): void {
    if (statusBarItem) {
        statusBarItem.hide();
    }
}

// ---------------------------------------------------------------------------
// Guard
// ---------------------------------------------------------------------------

function requireActiveReviewState(
    snapshot: ReviewCommandState,
): ReviewCommandState | undefined {
    return snapshot.active ? snapshot : undefined;
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/** Start review mode: parse %xrev tiers and begin stepping. */
export function startReview(): Effect.Effect<void, unknown, ExtensionCommandRequirements> {
    return Effect.gen(function*() {
        const editor = yield* requireActiveChatEditor();
        const window = yield* VSCodeWindowService;
        const commands = yield* VSCodeCommandsService;
        const reviewState = yield* ReviewCommandStateService;

        const text = editor.document.getText();
        const items = parseReviewItems(text);

        if (items.length === 0) {
            yield* Effect.asVoid(tryAsync('show no review items', () => Promise.resolve(
                window.showInformationMessage(
                    'No %xrev tiers found. Run `batchalign3 align --bullet-repair --review-level=low-confidence` first.',
                ),
            )));
            return;
        }

        yield* Effect.sync(() => {
            reviewState.activate(items, editor.document.uri.toString());
        });

        yield* Effect.asVoid(tryAsync('set review active context', () => Promise.resolve(
            commands.executeCommand('setContext', 'talkbank.reviewActive', true),
        )));

        updateStatusBar(items);

        // Navigate to first unreviewed item.
        const firstUnreviewed = nextUnreviewed(items, items.length - 1);
        if (firstUnreviewed >= 0) {
            yield* Effect.sync(() => {
                reviewState.setCurrentIndex(firstUnreviewed);
            });
            revealUtterance(editor, items[firstUnreviewed].utteranceLine);
        }

        yield* Effect.asVoid(tryAsync('show review started', () => Promise.resolve(
            window.showInformationMessage(
                `Review mode started: ${items.length} items, ${countReviewed(items)} already reviewed.`,
            ),
        )));
    });
}

/** Stop review mode. */
export function stopReview(): Effect.Effect<void, unknown, ExtensionCommandRequirements> {
    return Effect.gen(function*() {
        const commands = yield* VSCodeCommandsService;
        const reviewState = yield* ReviewCommandStateService;

        yield* Effect.sync(() => {
            reviewState.reset();
        });

        yield* Effect.asVoid(tryAsync('clear review context', () => Promise.resolve(
            commands.executeCommand('setContext', 'talkbank.reviewActive', false),
        )));

        hideStatusBar();
    });
}

/** Navigate to next unreviewed utterance. */
export function reviewNext(): Effect.Effect<void, unknown, ExtensionCommandRequirements> {
    return Effect.gen(function*() {
        const editor = yield* requireActiveChatEditor();
        const reviewState = yield* ReviewCommandStateService;
        const window = yield* VSCodeWindowService;
        const snapshot = requireActiveReviewState(reviewState.snapshot());

        if (!snapshot) {
            yield* Effect.asVoid(tryAsync('show not active', () => Promise.resolve(
                window.showWarningMessage('Review mode is not active. Run "Start Review Mode" first.'),
            )));
            return;
        }

        const next = nextUnreviewed(snapshot.items, snapshot.currentIndex);
        if (next < 0) {
            yield* Effect.asVoid(tryAsync('show all reviewed', () => Promise.resolve(
                window.showInformationMessage('All utterances have been reviewed!'),
            )));
            return;
        }

        yield* Effect.sync(() => {
            reviewState.setCurrentIndex(next);
        });
        revealUtterance(editor, snapshot.items[next].utteranceLine);
    });
}

/** Navigate to previous unreviewed utterance. */
export function reviewPrev(): Effect.Effect<void, unknown, ExtensionCommandRequirements> {
    return Effect.gen(function*() {
        const editor = yield* requireActiveChatEditor();
        const reviewState = yield* ReviewCommandStateService;
        const window = yield* VSCodeWindowService;
        const snapshot = requireActiveReviewState(reviewState.snapshot());

        if (!snapshot) {
            yield* Effect.asVoid(tryAsync('show not active', () => Promise.resolve(
                window.showWarningMessage('Review mode is not active.'),
            )));
            return;
        }

        const prev = prevUnreviewed(snapshot.items, snapshot.currentIndex);
        if (prev < 0) {
            yield* Effect.asVoid(tryAsync('show all reviewed', () => Promise.resolve(
                window.showInformationMessage('All utterances have been reviewed!'),
            )));
            return;
        }

        yield* Effect.sync(() => {
            reviewState.setCurrentIndex(prev);
        });
        revealUtterance(editor, snapshot.items[prev].utteranceLine);
    });
}

/**
 * Rate the current utterance and auto-advance.
 *
 * Each rating button calls this with its status value.
 */
export function rateUtterance(
    status: ReviewStatus,
): Effect.Effect<void, unknown, ExtensionCommandRequirements> {
    return Effect.gen(function*() {
        const editor = yield* requireActiveChatEditor();
        const reviewState = yield* ReviewCommandStateService;
        const window = yield* VSCodeWindowService;
        const snapshot = requireActiveReviewState(reviewState.snapshot());

        if (!snapshot) {
            return;
        }

        const item = snapshot.items[snapshot.currentIndex];
        if (!item || item.xrevLine < 0) {
            return;
        }

        // Edit the %xrev line in the document.
        const xrevLine = editor.document.lineAt(item.xrevLine);
        const newContent = `%xrev:\t${formatXrevContent(status, item.notes)}`;

        const editOk = yield* tryAsync('edit xrev tier', () =>
            Promise.resolve(editor.edit(editBuilder => {
                editBuilder.replace(xrevLine.range, newContent);
            })),
        );

        if (!editOk) {
            yield* Effect.asVoid(tryAsync('show edit failed', () => Promise.resolve(
                window.showErrorMessage('Failed to update %xrev tier.'),
            )));
            return;
        }

        // Update state.
        yield* Effect.sync(() => {
            reviewState.updateItemStatus(snapshot.currentIndex, status, item.notes);
        });

        // Update status bar.
        const updatedSnapshot = reviewState.snapshot();
        updateStatusBar(updatedSnapshot.items);

        // Auto-advance to next unreviewed.
        const next = nextUnreviewed(updatedSnapshot.items, snapshot.currentIndex);
        if (next >= 0) {
            yield* Effect.sync(() => {
                reviewState.setCurrentIndex(next);
            });
            revealUtterance(editor, updatedSnapshot.items[next].utteranceLine);
        } else {
            yield* Effect.asVoid(tryAsync('show all done', () => Promise.resolve(
                window.showInformationMessage(
                    `All ${updatedSnapshot.items.length} utterances reviewed! Save the file to preserve ratings.`,
                ),
            )));
        }
    });
}

/** Skip current utterance without rating — just advance to next. */
export function skipUtterance(): Effect.Effect<void, unknown, ExtensionCommandRequirements> {
    return reviewNext();
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Scroll the editor to reveal the given line and highlight it. */
function revealUtterance(editor: vscode.TextEditor, line: number): void {
    if (line < 0 || line >= editor.document.lineCount) {
        return;
    }
    const range = new vscode.Range(line, 0, line, 0);
    editor.selection = new vscode.Selection(range.start, range.start);
    editor.revealRange(range, vscode.TextEditorRevealType.InCenter);
}
