import assert from 'node:assert/strict';
import test from 'node:test';

const sourceRoot = process.env.TEST_SOURCE === 'src' ? '../src' : '../dist';
const {
    getRetryDelayMs,
    isRetryableScrappaError,
    ScrappaClient,
    ScrappaConnectionError,
    ScrappaHttpError,
    ScrappaTimeoutError,
} = await import(`${sourceRoot}/shared/scrappa-client.js`);

test('calculates capped exponential retry delays', () => {
    assert.equal(getRetryDelayMs(1, 0), 2_000);
    assert.equal(getRetryDelayMs(3, 500), 8_500);
    assert.equal(getRetryDelayMs(10, 999), 10_000);
});

test('classifies typed transient errors without parsing messages', () => {
    assert.equal(isRetryableScrappaError(new ScrappaTimeoutError(100)), true);
    assert.equal(isRetryableScrappaError(new ScrappaConnectionError()), true);
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(429, 'slow down')), true);
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(503, 'unavailable')), true);
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(400, 'invalid')), false);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (503): text only')), false);
});

test('retries a transient HTTP response and returns the successful response', async (context) => {
    const originalFetch = globalThis.fetch;
    const originalRandom = Math.random;
    context.after(() => {
        globalThis.fetch = originalFetch;
        Math.random = originalRandom;
    });

    Math.random = () => 0;
    let requestCount = 0;
    globalThis.fetch = async () => {
        requestCount++;
        return requestCount === 1
            ? new Response('Unavailable', { status: 503 })
            : Response.json({ suggestions: [] });
    };

    const client = new ScrappaClient({ apiKey: 'test-key' });
    const response = await client.get('/google-hotels/autocomplete', {}, { attempts: 2 });

    assert.deepEqual(response, { suggestions: [] });
    assert.equal(requestCount, 2);
});

test('preserves structured API errors in a typed HTTP error', async (context) => {
    const originalFetch = globalThis.fetch;
    context.after(() => { globalThis.fetch = originalFetch; });
    globalThis.fetch = async () => new Response(JSON.stringify({
        message: 'Invalid request',
        errors: { q: ['The query is required'] },
    }), { status: 422, headers: { 'content-type': 'application/json' } });

    const client = new ScrappaClient({ apiKey: 'test-key' });
    await assert.rejects(
        client.get('/google-hotels/autocomplete', {}, { attempts: 1 }),
        (error) => error instanceof ScrappaHttpError
            && error.status === 422
            && error.message.includes('q: The query is required'),
    );
});

test('joins base URLs and endpoint paths with one separator', async (context) => {
    const originalFetch = globalThis.fetch;
    context.after(() => { globalThis.fetch = originalFetch; });
    const requestedUrls = [];
    globalThis.fetch = async (url) => {
        requestedUrls.push(String(url));
        return Response.json({ suggestions: [] });
    };

    for (const [baseUrl, endpoint] of [
        ['http://localhost:3000/api', '/google-hotels/autocomplete'],
        ['http://localhost:3000/api/', '/google-hotels/autocomplete'],
        ['http://localhost:3000/api', 'google-hotels/autocomplete'],
        ['http://localhost:3000/api/', 'google-hotels/autocomplete'],
    ]) {
        const client = new ScrappaClient({ apiKey: 'test-key', baseUrl });
        await client.get(endpoint);
    }

    assert.deepEqual(requestedUrls, Array(4).fill('http://localhost:3000/api/google-hotels/autocomplete'));
});

test('wraps rejected fetch calls as retryable connection errors', async (context) => {
    const originalFetch = globalThis.fetch;
    context.after(() => { globalThis.fetch = originalFetch; });
    globalThis.fetch = async () => { throw new TypeError('fetch failed'); };

    const client = new ScrappaClient({ apiKey: 'test-key' });
    await assert.rejects(
        client.get('/google-hotels/autocomplete', {}, { attempts: 1 }),
        (error) => error instanceof ScrappaConnectionError
            && error.cause instanceof TypeError,
    );
});

test('converts aborted requests to timeout errors', async (context) => {
    const originalFetch = globalThis.fetch;
    context.after(() => { globalThis.fetch = originalFetch; });
    globalThis.fetch = async (_url, options) => new Promise((_resolve, reject) => {
        options.signal.addEventListener('abort', () => {
            const error = new Error('aborted');
            error.name = 'AbortError';
            reject(error);
        }, { once: true });
    });

    const client = new ScrappaClient({ apiKey: 'test-key', timeoutMs: 1 });
    await assert.rejects(
        client.get('/google-hotels/autocomplete', {}, { attempts: 1 }),
        (error) => error instanceof ScrappaTimeoutError,
    );
});
