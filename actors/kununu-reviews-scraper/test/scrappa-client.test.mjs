import assert from 'node:assert/strict';
import test from 'node:test';

import { ScrappaClient } from '../dist/shared/index.js';

test('encodes array query params for Laravel array validation', async () => {
    const originalFetch = globalThis.fetch;
    let requestedUrl;

    globalThis.fetch = async (url) => {
        requestedUrl = String(url);
        return new Response(JSON.stringify({ success: true }), {
            status: 200,
            headers: { 'content-type': 'application/json' },
        });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://scrappa.test/api',
        });

        await client.get('/kununu/reviews', {
            country: 'de',
            company_slug: 'bmwgroup',
            score_filters: ['excellent', 'good'],
        });

        const url = new URL(requestedUrl);
        assert.equal(url.searchParams.get('country'), 'de');
        assert.equal(url.searchParams.get('company_slug'), 'bmwgroup');
        assert.deepEqual(url.searchParams.getAll('score_filters[]'), ['excellent', 'good']);
    } finally {
        globalThis.fetch = originalFetch;
    }
});
