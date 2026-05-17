import assert from 'node:assert/strict';
import test from 'node:test';

import { ScrappaClient } from '../dist/shared/index.js';

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

        await client.get('/arbeitsagentur/jobs', {
            was: 'Software Entwickler',
            zeitarbeit: false,
        });
        await client.get('/arbeitsagentur/jobs', {
            was: 'Software Entwickler',
            zeitarbeit: true,
        });
    } finally {
        globalThis.fetch = originalFetch;
    }

    assert.equal(new URL(requestedUrls[0]).searchParams.get('zeitarbeit'), 'false');
    assert.equal(new URL(requestedUrls[1]).searchParams.get('zeitarbeit'), 'true');
});
