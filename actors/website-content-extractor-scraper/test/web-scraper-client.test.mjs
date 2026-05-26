import assert from 'node:assert/strict';
import test from 'node:test';

import {
    ScrappaWebScraperClient,
    ScrappaWebScraperHttpError,
} from '../dist/web-scraper-client.js';

test('calls Scrappa web-scraper with filtered GET params and json accept header', async () => {
    const originalFetch = globalThis.fetch;
    let capturedUrl;
    let capturedOptions;

    globalThis.fetch = async (url, options) => {
        capturedUrl = url;
        capturedOptions = options;
        return new Response(JSON.stringify({ success: true }), {
            status: 200,
            headers: { 'content-type': 'application/json' },
        });
    };

    try {
        const client = new ScrappaWebScraperClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        const result = await client.scrapeJson({
            url: 'https://example.com',
            include_html: false,
            response_type: 'json',
        });

        assert.deepEqual(result, { success: true });
        const url = new URL(capturedUrl);
        assert.equal(url.pathname, '/api/web-scraper');
        assert.equal(url.searchParams.get('url'), 'https://example.com');
        assert.equal(url.searchParams.get('response_type'), 'json');
        assert.equal(url.searchParams.has('include_html'), false);
        assert.equal(capturedOptions.headers['X-API-Key'], 'test-key');
        assert.equal(capturedOptions.headers.Accept, 'application/json');
        assert.equal(capturedOptions.headers['User-Agent'], 'thescrappa-website-content-extractor-scraper/1.0');
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('reads markdown responses as text', async () => {
    const originalFetch = globalThis.fetch;

    globalThis.fetch = async () => new Response('# Example Domain', {
        status: 200,
        headers: { 'content-type': 'text/markdown; charset=utf-8' },
    });

    try {
        const client = new ScrappaWebScraperClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        assert.equal(
            await client.scrapeMarkdown({
                url: 'https://example.com',
                response_type: 'markdown',
            }),
            '# Example Domain',
        );
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('parses Scrappa JSON errors and validation details', async () => {
    const originalFetch = globalThis.fetch;

    globalThis.fetch = async () => new Response(
        JSON.stringify({
            message: 'Invalid request',
            error_code: 'INVALID_URL',
            errors: {
                url: ['The url parameter is required.'],
            },
        }),
        {
            status: 400,
            statusText: 'Bad Request',
            headers: { 'content-type': 'application/json' },
        },
    );

    try {
        const client = new ScrappaWebScraperClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        await assert.rejects(
            () => client.scrapeJson({
                url: '',
                response_type: 'json',
            }),
            (error) => {
                assert.equal(error instanceof ScrappaWebScraperHttpError, true);
                assert.equal(error.status, 400);
                assert.equal(error.details, 'Invalid request - url: The url parameter is required.');
                assert.equal(error.body.error_code, 'INVALID_URL');
                return true;
            },
        );
    } finally {
        globalThis.fetch = originalFetch;
    }
});
