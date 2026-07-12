import assert from 'node:assert/strict';
import test from 'node:test';
import { isRetryableScrappaError, ScrappaClient, ScrappaHttpError } from '../dist/shared/scrappa-client.js';

test('classifies 429 as retryable and ordinary 4xx as non-retryable', () => {
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(429)), true);
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(404)), false);
});

test('retries HTTP 429 but stops after a non-retryable 4xx', async () => {
    const originalFetch = globalThis.fetch;
    let calls = 0;

    try {
        globalThis.fetch = async () => {
            calls += 1;
            return calls === 1
                ? new Response('', { status: 429 })
                : new Response(JSON.stringify({ ok: true }), { status: 200 });
        };

        const response = await new ScrappaClient({ apiKey: 'test' }).get('/test', {}, 2);
        assert.deepEqual(response, { ok: true });
        assert.equal(calls, 2);

        calls = 0;
        globalThis.fetch = async () => {
            calls += 1;
            return new Response('', { status: 404 });
        };

        await assert.rejects(
            () => new ScrappaClient({ apiKey: 'test' }).get('/test', {}, 3),
            ScrappaHttpError,
        );
        assert.equal(calls, 1);
    } finally {
        globalThis.fetch = originalFetch;
    }
});
