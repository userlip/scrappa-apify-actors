import assert from 'node:assert/strict';
import test from 'node:test';

const scrappaClientModule = process.env.TEST_SOURCE === 'src'
    ? '../src/shared/scrappa-client.ts'
    : '../dist/shared/scrappa-client.js';
const errorUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/shared/error-utils.ts'
    : '../dist/shared/error-utils.js';
const {
    isRetryableScrappaError,
    ScrappaTimeoutError,
} = await import(scrappaClientModule);
const { errorSummary } = await import(errorUtilsModule);

test('redacts credential-like values from upstream error summaries', () => {
    const summary = errorSummary('Bad gateway: {"token":"secret-value","api_key":"another-secret"}');
    assert.doesNotMatch(summary, /secret-value|another-secret/);
    assert.match(summary, /\[redacted\]/);
});

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
