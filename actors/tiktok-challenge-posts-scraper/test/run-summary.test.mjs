import assert from 'node:assert/strict';
import test from 'node:test';
import { isTotalFailure } from '../dist/run-summary.js';

test('fails a run only when every challenge failed without output', () => {
    assert.equal(isTotalFailure([
        { status: 'failed', videos_saved: 0 },
        { status: 'failed', videos_saved: 0 },
    ]), true);
    assert.equal(isTotalFailure([
        { status: 'failed', videos_saved: 0 },
        { status: 'succeeded', videos_saved: 0 },
    ]), false);
    assert.equal(isTotalFailure([
        { status: 'failed', videos_saved: 1 },
    ]), false);
    assert.equal(isTotalFailure([]), false);
});
