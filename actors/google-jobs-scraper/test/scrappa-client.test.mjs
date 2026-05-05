import assert from 'node:assert/strict';
import test from 'node:test';

import {
    getRetryDelayMs,
    isRetryableScrappaError,
    ScrappaTimeoutError,
} from '../dist/shared/scrappa-client.js';

test('uses capped exponential retry delay with jitter', () => {
    assert.equal(getRetryDelayMs(1, 250), 2250);
    assert.equal(getRetryDelayMs(2, 500), 4500);
    assert.equal(getRetryDelayMs(4, 750), 10000);
});

test('classifies transient Scrappa errors as retryable', () => {
    assert.equal(isRetryableScrappaError(new ScrappaTimeoutError(60000)), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (504): Gateway Timeout')), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (429): Too Many Requests')), true);
});

test('does not retry validation or unknown errors', () => {
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (400): Bad Request')), false);
    assert.equal(isRetryableScrappaError(new Error('Network failed')), false);
    assert.equal(isRetryableScrappaError('Scrappa API error (504): Gateway Timeout'), false);
});
