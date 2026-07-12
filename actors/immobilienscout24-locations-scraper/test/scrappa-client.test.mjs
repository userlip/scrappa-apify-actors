import assert from 'node:assert/strict';
import test from 'node:test';

const modulePath = process.env.TEST_SOURCE === 'src' ? '../src/shared/scrappa-client.ts' : '../dist/shared/scrappa-client.js';
const {
    getRetryDelayMs,
    isRetryableScrappaError,
    ScrappaApiError,
    ScrappaClient,
    ScrappaTimeoutError,
} = await import(modulePath);

test('calculates deterministic capped retry delays', () => {
    assert.equal(getRetryDelayMs(1, 250), 2250);
    assert.equal(getRetryDelayMs(4, 250), 10000);
});

test('retries transient HTTP, timeout, fetch, and network-cause failures', () => {
    assert.equal(isRetryableScrappaError(new ScrappaTimeoutError(1000)), true);
    assert.equal(isRetryableScrappaError(new ScrappaApiError(503, 'Unavailable')), true);
    assert.equal(isRetryableScrappaError(new ScrappaApiError(422, 'Invalid')), false);
    assert.equal(isRetryableScrappaError(new TypeError('fetch failed')), true);
    assert.equal(isRetryableScrappaError(new Error('request failed', { cause: { code: 'ECONNRESET' } })), true);
    assert.equal(isRetryableScrappaError(new Error('application error')), false);
});

test('calls only the Scrappa locations endpoint with query and limit', async () => {
    const originalFetch = globalThis.fetch;
    let requestedUrl;
    globalThis.fetch = async (url) => {
        requestedUrl = new URL(url);
        return new Response(JSON.stringify({ success: true, locations: [] }), { status: 200 });
    };

    try {
        const client = new ScrappaClient({ apiKey: 'test', timeoutMs: 1000 });
        await client.get('/immobilienscout24/locations', {
            query: 'Berlin',
            limit: 5,
            optional: undefined,
            empty: '',
        });
    } finally {
        globalThis.fetch = originalFetch;
    }

    assert.equal(requestedUrl.origin, 'https://scrappa.co');
    assert.equal(requestedUrl.pathname, '/api/immobilienscout24/locations');
    assert.equal(requestedUrl.searchParams.get('query'), 'Berlin');
    assert.equal(requestedUrl.searchParams.get('limit'), '5');
    assert.equal(requestedUrl.searchParams.has('optional'), false);
    assert.equal(requestedUrl.searchParams.has('empty'), false);
});

test('converts failed responses to typed Scrappa errors', async () => {
    const originalFetch = globalThis.fetch;
    globalThis.fetch = async () => new Response(JSON.stringify({ message: 'Invalid query' }), { status: 422 });

    try {
        const client = new ScrappaClient({ apiKey: 'test', timeoutMs: 1000 });
        await assert.rejects(
            client.get('/immobilienscout24/locations', { query: 'x', limit: 5 }),
            (error) => error instanceof ScrappaApiError && error.status === 422 && error.responseMessage === 'Invalid query',
        );
    } finally {
        globalThis.fetch = originalFetch;
    }
});
