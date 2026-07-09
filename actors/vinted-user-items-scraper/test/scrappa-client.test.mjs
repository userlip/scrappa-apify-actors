import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const scrappaClientModule = process.env.TEST_SOURCE === 'src'
    ? '../src/shared/scrappa-client.ts'
    : '../dist/shared/scrappa-client.js';
const {
    getRetryDelayMs,
    isRetryableScrappaError,
    ScrappaTimeoutError,
} = await import(scrappaClientModule);

test('starts exponential retry delays at the first failed attempt', () => {
    assert.equal(getRetryDelayMs(0, 0), 1000);
    assert.equal(getRetryDelayMs(1, 0), 2000);
    assert.equal(getRetryDelayMs(20, 0), 10000);
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
    assert.equal(isRetryableScrappaError(new Error('country must be one of: FR')), false);
    assert.equal(isRetryableScrappaError(new TypeError('invalid URL')), false);
    assert.equal(isRetryableScrappaError('fetch failed'), false);
});

test('keeps Vinted ScrappaClient implementations aligned except user agent', async () => {
    const normalizeClientSource = (source) => source.replace(
        /'User-Agent': 'thescrappa-vinted-[^']+'/,
        "'User-Agent': 'thescrappa-vinted-actor/1.0'",
    );
    const [userItemsClient, searchClient] = await Promise.all([
        readFile(new URL('../src/shared/scrappa-client.ts', import.meta.url), 'utf8'),
        readFile(new URL('../../vinted-search-scraper/src/shared/scrappa-client.ts', import.meta.url), 'utf8'),
    ]);

    assert.equal(normalizeClientSource(userItemsClient), normalizeClientSource(searchClient));
});
