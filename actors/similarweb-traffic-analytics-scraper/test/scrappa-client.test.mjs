import assert from 'node:assert/strict';
import test from 'node:test';

import {
    ScrappaHttpError,
    ScrappaTimeoutError,
    getRetryDelayMs,
    isRetryableScrappaError,
} from '../dist/shared/index.js';

test('classifies retryable Scrappa errors', () => {
    assert.equal(isRetryableScrappaError(new ScrappaTimeoutError(1000)), true);
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(408, 'Request timeout')), true);
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(429, 'Too many requests')), true);
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(503, 'Unavailable')), true);
    assert.equal(isRetryableScrappaError(new TypeError('fetch failed')), true);
});

test('classifies non-retryable Scrappa errors', () => {
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(400, 'Bad request')), false);
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(404, 'Not found')), false);
    assert.equal(isRetryableScrappaError(new Error('Validation failed')), false);
    assert.equal(isRetryableScrappaError('fetch failed'), false);
});

test('calculates capped retry delays with deterministic jitter', () => {
    assert.equal(getRetryDelayMs(1, 250), 2250);
    assert.equal(getRetryDelayMs(4, 250), 10000);
});
