import assert from 'node:assert/strict';
import test from 'node:test';

import { ScrappaClient } from '../dist/shared/scrappa-client.js';

test('does not send use_cache=0 when use_cache is false', async () => {
    const originalFetch = globalThis.fetch;
    let capturedUrl = '';

    globalThis.fetch = async (url) => {
        capturedUrl = String(url);
        return new Response(JSON.stringify({ items: [] }), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
        });
    };

    try {
        const client = new ScrappaClient({ apiKey: 'test', baseUrl: 'https://example.com/api' });
        await client.get('/maps/simple-search', {
            query: 'pizza',
            use_cache: false,
            hl: 'en',
        });
    } finally {
        globalThis.fetch = originalFetch;
    }

    assert.ok(capturedUrl.includes('query=pizza'));
    assert.ok(capturedUrl.includes('hl=en'));
    assert.ok(!capturedUrl.includes('use_cache=0'));
});

test('surfaces non-JSON upstream errors without body-read crash', async () => {
    const originalFetch = globalThis.fetch;

    globalThis.fetch = async () => {
        return new Response('<html><body>upstream timeout</body></html>', {
            status: 524,
            headers: { 'Content-Type': 'text/html' },
        });
    };

    try {
        const client = new ScrappaClient({ apiKey: 'test', baseUrl: 'https://example.com/api' });
        await assert.rejects(
            () => client.get('/maps/simple-search', { query: 'pizza' }),
            /Scrappa API error \(524\): .*upstream timeout/i
        );
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('aborts Scrappa requests after the configured timeout', async () => {
    const originalFetch = globalThis.fetch;
    let capturedSignal;

    globalThis.fetch = async (_url, options = {}) => {
        const signal = options.signal;
        capturedSignal = signal;
        assert.ok(signal instanceof AbortSignal);

        return new Promise((_resolve, reject) => {
            signal.addEventListener('abort', () => {
                reject(new DOMException('The operation was aborted.', 'AbortError'));
            });
        });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test',
            baseUrl: 'https://example.com/api',
            timeoutMs: 10,
        });

        await assert.rejects(
            () => client.get('/maps/simple-search', { query: 'pizza' }),
            /Scrappa API request timed out after 10ms/
        );
        assert.ok(capturedSignal.aborted);
    } finally {
        globalThis.fetch = originalFetch;
    }
});
