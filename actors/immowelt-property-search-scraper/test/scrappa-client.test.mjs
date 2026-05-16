import assert from 'node:assert/strict';
import test from 'node:test';

const scrappaClientModule = process.env.TEST_SOURCE === 'src'
    ? '../src/shared/index.ts'
    : '../dist/shared/index.js';
const { ScrappaClient } = await import(scrappaClientModule);

test('serializes true and false boolean GET parameters', async () => {
    const originalFetch = globalThis.fetch;
    const requestedUrls = [];

    globalThis.fetch = async (url) => {
        requestedUrls.push(String(url));

        return new Response(JSON.stringify({ success: true }), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
        });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        await client.get('/immowelt/search', {
            location: 'Berlin',
            include_private: false,
        });
        await client.get('/immowelt/search', {
            location: 'Berlin',
            include_private: true,
        });
    } finally {
        globalThis.fetch = originalFetch;
    }

    assert.equal(new URL(requestedUrls[0]).searchParams.get('include_private'), 'false');
    assert.equal(new URL(requestedUrls[1]).searchParams.get('include_private'), 'true');
});
