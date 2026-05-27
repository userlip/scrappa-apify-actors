import assert from 'node:assert/strict';
import test from 'node:test';

import { ScrappaClient } from '../dist/shared/index.js';

test('serializes boolean GET parameters as Laravel-compatible boolean values', async () => {
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

        await client.get('/stepstone/jobs', {
            query: 'software engineer',
            work_from_home: false,
        });
        await client.get('/stepstone/jobs', {
            query: 'software engineer',
            work_from_home: true,
        });
    } finally {
        globalThis.fetch = originalFetch;
    }

    assert.equal(new URL(requestedUrls[0]).searchParams.get('work_from_home'), 'false');
    assert.equal(new URL(requestedUrls[1]).searchParams.get('work_from_home'), 'true');
});
