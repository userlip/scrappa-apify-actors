import assert from 'node:assert/strict';
import test from 'node:test';

import {
    getRetryDelayMs,
    isRetryableScrappaError,
    ScrappaHttpError,
    ScrappaTimeoutError,
} from '../dist/shared/scrappa-client.js';

test('identifies retryable Scrappa API failures', () => {
    assert.equal(isRetryableScrappaError(new ScrappaTimeoutError(1000)), true);
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(429, 'Too many requests')), true);
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(503, 'Service unavailable')), true);
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(404, 'Not found')), false);
});

test('retries low-level fetch network failures', () => {
    assert.equal(isRetryableScrappaError(new TypeError('fetch failed')), true);
    assert.equal(isRetryableScrappaError(new TypeError('Failed to fetch')), true);
    assert.equal(isRetryableScrappaError(new TypeError('terminated')), true);
    assert.equal(isRetryableScrappaError(new TypeError('read ECONNRESET')), true);
    assert.equal(isRetryableScrappaError(new TypeError('socket hang up')), true);
    assert.equal(isRetryableScrappaError(new TypeError('Cannot convert undefined or null to object')), false);
});

test('caps retry delay with jitter', () => {
    assert.equal(getRetryDelayMs(1, 0), 2000);
    assert.equal(getRetryDelayMs(20, 500), 10000);
});
