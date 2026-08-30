import assert from 'node:assert/strict';
import test from 'node:test';

import { ScrappaClient, ScrappaHttpError } from '../dist/shared/scrappa-client.js';

test('retries a transient response once', async () => {
    const originalFetch = globalThis.fetch;
    const delays = [];
    let calls = 0;

    globalThis.fetch = async () => {
        calls += 1;
        return calls === 1
            ? new Response('', { status: 502 })
            : new Response(JSON.stringify([{ position: 1 }]), { status: 200 });
    };

    try {
        const client = new ScrappaClient({ apiKey: 'test', baseUrl: 'https://example.test/api' });
        const response = await client.get('/images', { q: 'coffee product photography' }, {
            attempts: 2,
            retryDelayMs(failedAttempt) {
                delays.push(failedAttempt);
                return 0;
            },
        });

        assert.deepEqual(response, [{ position: 1 }]);
        assert.equal(calls, 2);
        assert.deepEqual(delays, [1]);
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('does not retry invalid requests', async () => {
    const originalFetch = globalThis.fetch;
    let calls = 0;

    globalThis.fetch = async () => {
        calls += 1;
        return new Response(JSON.stringify({ message: 'Invalid query' }), { status: 400 });
    };

    try {
        const client = new ScrappaClient({ apiKey: 'test', baseUrl: 'https://example.test/api' });
        await assert.rejects(
            () => client.get('/images', { q: 'invalid' }, { attempts: 2, retryDelayMs: () => 0 }),
            (error) => error instanceof ScrappaHttpError && error.status === 400,
        );
        assert.equal(calls, 1);
    } finally {
        globalThis.fetch = originalFetch;
    }
});
