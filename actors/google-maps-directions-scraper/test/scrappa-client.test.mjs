import assert from 'node:assert/strict';
import test from 'node:test';

const modulePath = process.env.TEST_SOURCE === 'src'
    ? '../src/shared/scrappa-client.ts'
    : '../dist/shared/scrappa-client.js';
const { ScrappaClient, ScrappaTimeoutError, getRetryDelayMs, isRetryableScrappaError } = await import(modulePath);

test('builds the Scrappa directions request with only defined parameters', async () => {
    const originalFetch = globalThis.fetch;
    let captured;
    globalThis.fetch = async (url, options) => {
        captured = { url: String(url), options };
        return new Response(JSON.stringify({ status: 'OK', directions: [] }), { status: 200, headers: { 'content-type': 'application/json' } });
    };

    try {
        const client = new ScrappaClient({ apiKey: 'test-key', baseUrl: 'https://example.test/api' });
        await client.get('/google-maps-directions', { origin: 'A', destination: 'B', mode: 'driving', hl: 'en', gl: '' });
    } finally {
        globalThis.fetch = originalFetch;
    }

    assert.equal(captured.url, 'https://example.test/api/google-maps-directions?origin=A&destination=B&mode=driving&hl=en');
    assert.equal(captured.options.headers['X-API-Key'], 'test-key');
});

test('recognizes retryable failures and uses bounded retry delay', () => {
    assert.equal(isRetryableScrappaError(new ScrappaTimeoutError(1000)), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (429): Too many requests')), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (400): Bad request')), false);
    assert.equal(getRetryDelayMs(3, 0), 8000);
    assert.equal(getRetryDelayMs(5, 0), 10000);
});

test('retries a transient Scrappa response and succeeds on the next attempt', async () => {
    const originalFetch = globalThis.fetch;
    let attempts = 0;
    globalThis.fetch = async () => {
        attempts += 1;
        if (attempts === 1) {
            return new Response('temporary failure', { status: 503, statusText: 'Service Unavailable' });
        }
        return new Response(JSON.stringify({ status: 'OK', directions: [{ distance: 1 }] }), { status: 200 });
    };

    try {
        const client = new ScrappaClient({ apiKey: 'test-key', baseUrl: 'https://example.test/api', retryDelayMs: 0 });
        const response = await client.get('/google-maps-directions', { origin: 'A', destination: 'B' }, { attempts: 2 });
        assert.deepEqual(response, { status: 'OK', directions: [{ distance: 1 }] });
    } finally {
        globalThis.fetch = originalFetch;
    }

    assert.equal(attempts, 2);
});
