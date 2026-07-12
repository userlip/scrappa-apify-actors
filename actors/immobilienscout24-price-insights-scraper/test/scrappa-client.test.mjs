import assert from 'node:assert/strict';
import test from 'node:test';
import { isRetryableScrappaError } from '../dist/shared/index.js';

test('retries transient fetch and network errors', () => {
    assert.equal(isRetryableScrappaError(new TypeError('fetch failed')), true);
    assert.equal(isRetryableScrappaError(new Error('request failed', { cause: { code: 'ENOTFOUND' } })), true);
    assert.equal(isRetryableScrappaError(new Error('request failed', { cause: { code: 'EAI_AGAIN' } })), true);
});

test('does not retry unrelated application errors', () => {
    assert.equal(isRetryableScrappaError(new Error('invalid location')), false);
});
