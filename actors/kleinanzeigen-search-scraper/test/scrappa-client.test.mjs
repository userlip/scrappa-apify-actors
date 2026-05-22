import assert from 'node:assert/strict';
import test from 'node:test';

const scrappaClientModule = process.env.TEST_SOURCE === 'src'
    ? '../src/shared/scrappa-client.ts'
    : '../dist/shared/scrappa-client.js';
const {
    isRetryableScrappaError,
    ScrappaTimeoutError,
} = await import(scrappaClientModule);

test('retries timeout, transient API, and fetch transport errors', () => {
    assert.equal(isRetryableScrappaError(new ScrappaTimeoutError(1000)), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (429): Too many requests')), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (503): Service unavailable')), true);
    assert.equal(isRetryableScrappaError(new TypeError('fetch failed')), true);
    assert.equal(
        isRetryableScrappaError(new TypeError('request failed', {
            cause: new Error('connect ECONNRESET 127.0.0.1:443'),
        })),
        true,
    );
});

test('does not retry validation or non-transient Scrappa errors', () => {
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (400): Bad request')), false);
    assert.equal(isRetryableScrappaError(new Error('query is required')), false);
    assert.equal(isRetryableScrappaError(new TypeError('invalid URL')), false);
    assert.equal(isRetryableScrappaError('fetch failed'), false);
});
