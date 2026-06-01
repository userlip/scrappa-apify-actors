import assert from 'node:assert/strict';
import test from 'node:test';

import { ScrappaClient, ScrappaNetworkError } from '../dist/shared/index.js';

test('ScrappaClient builds the domain availability request with actor user agent', async () => {
    const originalFetch = globalThis.fetch;
    let capturedUrl;
    let capturedOptions;

    globalThis.fetch = async (url, options) => {
        capturedUrl = url;
        capturedOptions = options;
        return new Response(JSON.stringify({ domain: 'example.com' }), { status: 200 });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        const result = await client.get('/domains/availability', { domain: 'example.com' });

        assert.deepEqual(result, { domain: 'example.com' });
        const url = new URL(capturedUrl);
        assert.equal(url.pathname, '/api/domains/availability');
        assert.equal(url.searchParams.get('domain'), 'example.com');
        assert.equal(capturedOptions.headers['X-API-Key'], 'test-key');
        assert.equal(capturedOptions.headers['User-Agent'], 'thescrappa-domain-availability-checker/1.0');
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('ScrappaClient retries transient network fetch failures', async () => {
    const originalFetch = globalThis.fetch;
    let attempts = 0;

    globalThis.fetch = async () => {
        attempts += 1;
        if (attempts === 1) {
            throw new TypeError('fetch failed');
        }

        return new Response(JSON.stringify({ domain: 'example.com' }), { status: 200 });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        const result = await client.get('/domains/availability', { domain: 'example.com' }, { attempts: 2 });

        assert.deepEqual(result, { domain: 'example.com' });
        assert.equal(attempts, 2);
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('ScrappaClient exposes persistent network fetch failures as ScrappaNetworkError', async () => {
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
            () => client.get('/domains/availability', { domain: 'example.com' }, { attempts: 1 }),
            (error) => error instanceof ScrappaNetworkError && error.message === 'Scrappa API network error: fetch failed',
        );
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('ScrappaClient preserves false boolean query params', async () => {
    const originalFetch = globalThis.fetch;
    let capturedUrl;

    globalThis.fetch = async (url) => {
        capturedUrl = url;
        return new Response(JSON.stringify({ ok: true }), { status: 200 });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        await client.get('/domains/availability', { domain: 'example.com', debug: false });

        const url = new URL(capturedUrl);
        assert.equal(url.searchParams.get('debug'), 'false');
    } finally {
        globalThis.fetch = originalFetch;
    }
});
