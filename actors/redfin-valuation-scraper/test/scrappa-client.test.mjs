import assert from 'node:assert/strict';
import test from 'node:test';

import {
    ScrappaClient,
    ScrappaHttpError,
} from '../dist/shared/index.js';

test('builds valuation GET requests with actor user agent', async () => {
    const originalFetch = globalThis.fetch;
    let capturedUrl;
    let capturedOptions;

    globalThis.fetch = async (url, options) => {
        capturedUrl = url;
        capturedOptions = options;
        return new Response(JSON.stringify({ data: { predictedValue: 850000 } }), { status: 200 });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        const result = await client.get('/redfin/valuation', {
            property_id: 194191988,
            listing_id: 207388793,
            missing: undefined,
        });

        assert.deepEqual(result, { data: { predictedValue: 850000 } });
        const url = new URL(capturedUrl);
        assert.equal(url.pathname, '/api/redfin/valuation');
        assert.equal(url.searchParams.get('property_id'), '194191988');
        assert.equal(url.searchParams.get('listing_id'), '207388793');
        assert.equal(capturedOptions.headers['X-API-Key'], 'test-key');
        assert.equal(capturedOptions.headers['User-Agent'], 'thescrappa-redfin-valuation-scraper/1.0');
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('parses Scrappa JSON error messages', async () => {
    const originalFetch = globalThis.fetch;

    globalThis.fetch = async () => new Response(
        JSON.stringify({
            message: 'Invalid request',
            errors: {
                property_id: ['The property ID is required.'],
            },
        }),
        { status: 422, statusText: 'Unprocessable Entity' },
    );

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        await assert.rejects(
            () => client.get('/redfin/valuation'),
            (error) => {
                assert.equal(error instanceof ScrappaHttpError, true);
                assert.equal(error.status, 422);
                assert.equal(error.details, 'Invalid request - property_id: The property ID is required.');
                return true;
            },
        );
    } finally {
        globalThis.fetch = originalFetch;
    }
});
