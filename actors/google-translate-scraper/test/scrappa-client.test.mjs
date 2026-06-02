import assert from 'node:assert/strict';
import test from 'node:test';

const sharedModule = process.env.TEST_SOURCE === 'src'
    ? '../src/shared/index.ts'
    : '../dist/shared/index.js';
const {
    ScrappaClient,
    ScrappaHttpError,
    ScrappaNetworkError,
} = await import(sharedModule);

test('ScrappaClient builds the Google Translate request with actor user agent', async () => {
    const originalFetch = globalThis.fetch;
    let capturedUrl;
    let capturedOptions;

    globalThis.fetch = async (url, options) => {
        capturedUrl = url;
        capturedOptions = options;
        return new Response(JSON.stringify({ translated_text: 'Hallo' }), { status: 200 });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        const result = await client.get('/google-translate', {
            text: 'Good morning',
            source: 'en',
            target: 'de',
        });

        assert.deepEqual(result, { translated_text: 'Hallo' });
        const url = new URL(capturedUrl);
        assert.equal(url.pathname, '/api/google-translate');
        assert.equal(url.searchParams.get('text'), 'Good morning');
        assert.equal(url.searchParams.get('source'), 'en');
        assert.equal(url.searchParams.get('target'), 'de');
        assert.equal(url.searchParams.has('append'), false);
        assert.equal(url.searchParams.has('html'), false);
        assert.equal(capturedOptions.headers['X-API-Key'], 'test-key');
        assert.equal(capturedOptions.headers['User-Agent'], 'thescrappa-google-translate-scraper/1.0');
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('ScrappaClient surfaces JSON error fields from Scrappa', async () => {
    const originalFetch = globalThis.fetch;

    globalThis.fetch = async () => new Response(
        JSON.stringify({ error: 'Translation service temporarily unavailable. Please retry.' }),
        { status: 503 },
    );

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        await assert.rejects(
            () => client.get('/google-translate', { text: 'Hello', source: 'en', target: 'de' }, { attempts: 1 }),
            (error) => error instanceof ScrappaHttpError
                && error.status === 503
                && error.details === 'Translation service temporarily unavailable. Please retry.',
        );
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('ScrappaClient exposes persistent network failures as ScrappaNetworkError', async () => {
    const originalFetch = globalThis.fetch;

    globalThis.fetch = async () => {
        throw new TypeError('fetch failed');
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        await assert.rejects(
            () => client.get('/google-translate', { text: 'Hello', source: 'en', target: 'de' }, { attempts: 1 }),
            (error) => error instanceof ScrappaNetworkError && error.message === 'Scrappa API network error: fetch failed',
        );
    } finally {
        globalThis.fetch = originalFetch;
    }
});
