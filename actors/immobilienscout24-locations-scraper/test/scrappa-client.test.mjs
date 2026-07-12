import assert from 'node:assert/strict';
import test from 'node:test';

const modulePath = process.env.TEST_SOURCE === 'src' ? '../src/shared/scrappa-client.ts' : '../dist/shared/scrappa-client.js';
const { ScrappaApiError, ScrappaClient } = await import(modulePath);

test('calls only the Scrappa locations endpoint with query and limit', async () => {
    const originalFetch = globalThis.fetch;
    let requestedUrl;
    globalThis.fetch = async (url) => {
        requestedUrl = new URL(url);
        return new Response(JSON.stringify({ success: true, locations: [] }), { status: 200 });
    };

    try {
        const client = new ScrappaClient({ apiKey: 'test', timeoutMs: 1000 });
        await client.get('/immobilienscout24/locations', { query: 'Berlin', limit: 5 });
    } finally {
        globalThis.fetch = originalFetch;
    }

    assert.equal(requestedUrl.origin, 'https://scrappa.co');
    assert.equal(requestedUrl.pathname, '/api/immobilienscout24/locations');
    assert.equal(requestedUrl.searchParams.get('query'), 'Berlin');
    assert.equal(requestedUrl.searchParams.get('limit'), '5');
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
