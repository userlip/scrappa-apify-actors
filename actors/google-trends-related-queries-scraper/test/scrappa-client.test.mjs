import assert from 'node:assert/strict';
import test from 'node:test';

import {
    ScrappaTimeoutError,
    getRetryDelayMs,
    isRetryableScrappaError,
} from '../dist/shared/index.js';

test('classifies retryable Scrappa API errors', () => {
    assert.equal(isRetryableScrappaError(new ScrappaTimeoutError(1000)), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (408): Timeout')), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (429): Too Many Requests')), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (500): Server Error')), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (503): Service Unavailable')), true);
});

test('does not retry validation, auth, or unrelated errors', () => {
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (400): Bad Request')), false);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (401): Unauthorized')), false);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (422): Validation failed')), false);
    assert.equal(isRetryableScrappaError(new Error('network failed')), false);
    assert.equal(isRetryableScrappaError('Scrappa API error (500): Server Error'), false);
});

test('calculates capped exponential retry delay', () => {
    assert.equal(getRetryDelayMs(1, 0), 2000);
    assert.equal(getRetryDelayMs(2, 250), 4250);
    assert.equal(getRetryDelayMs(10, 0), 10000);
});
