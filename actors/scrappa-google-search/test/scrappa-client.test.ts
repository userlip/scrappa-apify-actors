import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ScrappaClient } from '../src/shared/scrappa-client.js';

test('GET retries a transient 503 without changing the search request', async () => {
    const originalFetch = globalThis.fetch;
    const calls: string[] = [];
    globalThis.fetch = async (input) => {
        calls.push(String(input));
        return calls.length === 1
            ? new Response('', { status: 503 })
            : Response.json({ organic_results: [{ title: 'Result' }] });
    };

    try {
        const client = new ScrappaClient({ apiKey: 'test', baseUrl: 'https://example.test/api' });
        const response = await client.get<{ organic_results: { title: string }[] }>('/search', { query: 'restaurants', amount: 10 });
        assert.equal(response.organic_results[0].title, 'Result');
        assert.deepEqual(calls, [
            'https://example.test/api/search?query=restaurants&amount=10',
            'https://example.test/api/search?query=restaurants&amount=10',
        ]);
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('GET does not retry permanent errors', async () => {
    const originalFetch = globalThis.fetch;
    let calls = 0;
    globalThis.fetch = async () => {
        calls++;
        return new Response('', { status: 401 });
    };

    try {
        const client = new ScrappaClient({ apiKey: 'test', baseUrl: 'https://example.test/api' });
        await assert.rejects(client.get('/search', { query: 'restaurants' }), /Scrappa API error \(401\)/);
        assert.equal(calls, 1);
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('GET fails after three transient 503 responses', async () => {
    const originalFetch = globalThis.fetch;
    let calls = 0;
    globalThis.fetch = async () => {
        calls++;
        return new Response('', { status: 503 });
    };

    try {
        const client = new ScrappaClient({ apiKey: 'test', baseUrl: 'https://example.test/api' });
        await assert.rejects(client.get('/search', { query: 'restaurants' }), /Scrappa API error \(503\)/);
        assert.equal(calls, 3);
    } finally {
        globalThis.fetch = originalFetch;
    }
});
