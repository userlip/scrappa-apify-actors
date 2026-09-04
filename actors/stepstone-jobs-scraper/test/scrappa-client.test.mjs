import assert from 'node:assert/strict';
import test from 'node:test';

import {
    ScrappaApiError,
    ScrappaClient,
    ScrappaTimeoutError,
    getRetryDelayMs,
    isRetryableScrappaError,
} from '../dist/shared/index.js';

const originalFetch = globalThis.fetch;
const originalSetTimeout = globalThis.setTimeout;
const originalWarn = console.warn;

test.afterEach(() => {
    globalThis.fetch = originalFetch;
    globalThis.setTimeout = originalSetTimeout;
    console.warn = originalWarn;
});

test('retries timeouts and transient Scrappa API errors only', () => {
    assert.equal(isRetryableScrappaError(new ScrappaTimeoutError(1000)), true);
    assert.equal(isRetryableScrappaError(new ScrappaApiError(429, 'Too Many Requests')), true);
    assert.equal(isRetryableScrappaError(new ScrappaApiError(503, 'Service Unavailable')), true);
    assert.equal(isRetryableScrappaError(new ScrappaApiError(400, 'Bad Request')), false);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (503): forged')), false);
});

test('calculates bounded retry delays that honor Retry-After', () => {
    assert.equal(getRetryDelayMs(1, 0), 2000);
    assert.equal(getRetryDelayMs(2, 250), 4250);
    assert.equal(getRetryDelayMs(1, 0, 15000), 15000);
    assert.equal(getRetryDelayMs(1, 0, 60000, 20000), 20000);
});

test('serializes true and false boolean GET parameters', async () => {
    const requestedUrls = [];

    globalThis.fetch = async (url) => {
        requestedUrls.push(String(url));

        return new Response(JSON.stringify({ success: true }), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
        });
    };

    const client = new ScrappaClient({
        apiKey: 'test-key',
        baseUrl: 'https://example.test/api',
    });

    await client.get('/stepstone/jobs', {
        query: 'Software Entwickler',
        work_from_home: false,
    });
    await client.get('/stepstone/jobs', {
        query: 'Software Entwickler',
        work_from_home: true,
    });

    assert.equal(new URL(requestedUrls[0]).searchParams.get('work_from_home'), '0');
    assert.equal(new URL(requestedUrls[1]).searchParams.get('work_from_home'), '1');
});

test('honors and caps Retry-After during a retry', async () => {
    let callCount = 0;
    const retryDelays = [];
    console.warn = () => {};
    globalThis.fetch = async () => {
        callCount += 1;
        if (callCount === 1) {
            return new Response(JSON.stringify({ message: 'temporarily busy' }), {
                status: 503,
                headers: {
                    'Content-Type': 'application/json',
                    'Retry-After': '60',
                },
            });
        }

        return new Response(JSON.stringify({ success: true }), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
        });
    };
    globalThis.setTimeout = (callback, delay, ...args) => {
        if (delay === 20000) {
            retryDelays.push(delay);
            queueMicrotask(() => callback(...args));
            return 0;
        }

        return originalSetTimeout(callback, delay, ...args);
    };

    const client = new ScrappaClient({
        apiKey: 'test-key',
        baseUrl: 'https://example.test/api',
        maxRetryDelayMs: 20000,
        timeoutMs: 1000,
    });

    assert.deepEqual(await client.get('/stepstone/jobs', {}, { attempts: 2 }), { success: true });
    assert.equal(callCount, 2);
    assert.deepEqual(retryDelays, [20000]);
});

test('recovers from three consecutive 503s without exceeding the configured attempt limit', async () => {
    let calls = 0;
    console.warn = () => {};
    globalThis.fetch = async () => {
        calls++;
        return calls <= 3
            ? new Response(JSON.stringify({ error: { message: 'Stepstone upstream is temporarily unavailable.' } }), { status: 503 })
            : new Response(JSON.stringify({ data: { jobs: [{ title: 'Engineer' }] } }));
    };
    globalThis.setTimeout = (callback, delay, ...args) => {
        if (delay <= 20000) { queueMicrotask(() => callback(...args)); return 0; }
        return originalSetTimeout(callback, delay, ...args);
    };
    const client = new ScrappaClient({ apiKey: 'test', timeoutMs: 30000, maxRetryDelayMs: 20000 });
    const result = await client.get('/stepstone/jobs', {}, { attempts: 4 });
    assert.equal(calls, 4);
    assert.equal(result.data.jobs.length, 1);
});

test('preserves nested upstream errors and stops after four failures', async () => {
    let calls = 0;
    console.warn = () => {};
    globalThis.fetch = async () => {
        calls++;
        return new Response(JSON.stringify({ error: { message: 'Stepstone upstream is temporarily unavailable.' } }), { status: 503 });
    };
    globalThis.setTimeout = (callback, delay, ...args) => {
        if (delay <= 20000) { queueMicrotask(() => callback(...args)); return 0; }
        return originalSetTimeout(callback, delay, ...args);
    };
    const client = new ScrappaClient({ apiKey: 'test', timeoutMs: 30000, maxRetryDelayMs: 20000 });
    await assert.rejects(client.get('/stepstone/jobs', {}, { attempts: 4 }), /Stepstone upstream is temporarily unavailable/);
    assert.equal(calls, 4);
});
