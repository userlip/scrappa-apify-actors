import assert from 'node:assert/strict';
import test from 'node:test';

import { getRetryDelayMs, isRetryableScrappaError, ScrappaTimeoutError } from '../dist/shared/index.js';

test('detects retryable Scrappa HTTP errors', () => {
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (429): Too many requests')), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (503): Service unavailable')), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (422): Invalid query')), false);
});

test('detects Scrappa timeout errors as retryable', () => {
    assert.equal(isRetryableScrappaError(new ScrappaTimeoutError(1000)), true);
});

test('caps retry delay at ten seconds', () => {
    assert.equal(getRetryDelayMs(10, 0), 10000);
});
