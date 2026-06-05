import assert from 'node:assert/strict';
import test from 'node:test';

import {
    ScrappaClient,
    ScrappaTimeoutError,
    getRetryDelayMs,
    isRetryableScrappaError,
} from '../dist/shared/index.js';

const originalFetch = globalThis.fetch;
const originalRandom = Math.random;
const originalWarn = console.warn;

test.afterEach(() => {
    globalThis.fetch = originalFetch;
    Math.random = originalRandom;
    console.warn = originalWarn;
});

test('classifies retryable Scrappa API errors', () => {
    assert.equal(isRetryableScrappaError(new ScrappaTimeoutError(1000)), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (408): Timeout')), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (429): Too Many Requests')), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (500): Server Error')), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (503): Service Unavailable')), true);
});

test('does not retry validation, auth, or unrelated errors', () => {
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (400): Bad Request')), false);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (401): Unauthorized')), false);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (422): Validation failed')), false);
    assert.equal(isRetryableScrappaError(new Error('network failed')), false);
    assert.equal(isRetryableScrappaError('Scrappa API error (500): Server Error'), false);
});

test('calculates capped exponential retry delay', () => {
    assert.equal(getRetryDelayMs(1, 0), 2000);
    assert.equal(getRetryDelayMs(2, 250), 4250);
    assert.equal(getRetryDelayMs(10, 0), 10000);
});

test('sends GET requests with cleaned query parameters and API headers', async () => {
    const calls = [];
    globalThis.fetch = async (url, options) => {
        calls.push({ url, options });
        return new Response(JSON.stringify({ ok: true }), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
        });
    };

    const client = new ScrappaClient({
        apiKey: 'test-key',
        baseUrl: 'https://scrappa.test/api',
        timeoutMs: 1000,
    });

    const response = await client.get('/google-trends/autocomplete', {
        q: 'coffee',
        geo: 'US',
        include_cache: true,
        skip_cache: false,
        empty: '',
    });

    assert.deepEqual(response, { ok: true });
    assert.equal(calls.length, 1);

    const url = new URL(calls[0].url);
    assert.equal(url.origin + url.pathname, 'https://scrappa.test/api/google-trends/autocomplete');
    assert.equal(url.searchParams.get('q'), 'coffee');
    assert.equal(url.searchParams.get('geo'), 'US');
    assert.equal(url.searchParams.get('include_cache'), '1');
    assert.equal(url.searchParams.has('skip_cache'), false);
    assert.equal(url.searchParams.has('empty'), false);
    assert.equal(calls[0].options.method, 'GET');
    assert.equal(calls[0].options.headers['X-API-Key'], 'test-key');
    assert.equal(calls[0].options.headers.Accept, 'application/json');
});

test('sends POST requests with JSON body and API headers', async () => {
    const calls = [];
    globalThis.fetch = async (url, options) => {
        calls.push({ url, options });
        return new Response(JSON.stringify({ created: true }), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
        });
    };

    const client = new ScrappaClient({
        apiKey: 'test-key',
        baseUrl: 'https://scrappa.test/api',
        timeoutMs: 1000,
    });

    const response = await client.post('/google-trends/autocomplete', { q: 'coffee' });

    assert.deepEqual(response, { created: true });
    assert.equal(calls.length, 1);
    assert.equal(calls[0].url, 'https://scrappa.test/api/google-trends/autocomplete');
    assert.equal(calls[0].options.method, 'POST');
    assert.equal(calls[0].options.headers['Content-Type'], 'application/json');
    assert.equal(calls[0].options.body, JSON.stringify({ q: 'coffee' }));
});

test('retries retryable Scrappa API responses before returning success', async () => {
    const calls = [];
    Math.random = () => -2;
    console.warn = () => {};
    globalThis.fetch = async (url, options) => {
        calls.push({ url, options });
        if (calls.length === 1) {
            return new Response(JSON.stringify({ message: 'temporarily busy' }), {
                status: 503,
                statusText: 'Service Unavailable',
                headers: { 'Content-Type': 'application/json' },
            });
        }

        return new Response(JSON.stringify({ ok: true }), {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
        });
    };

    const client = new ScrappaClient({
        apiKey: 'test-key',
        baseUrl: 'https://scrappa.test/api',
        timeoutMs: 1000,
    });

    const response = await client.get('/retry', {}, { attempts: 2 });

    assert.deepEqual(response, { ok: true });
    assert.equal(calls.length, 2);
});

test('uses status text when an error response body cannot be read', async () => {
    globalThis.fetch = async () => ({
        ok: false,
        status: 500,
        statusText: 'Server Error',
        text: async () => {
            throw new Error('body stream failed');
        },
    });

    const client = new ScrappaClient({
        apiKey: 'test-key',
        baseUrl: 'https://scrappa.test/api',
        timeoutMs: 1000,
    });

    await assert.rejects(
        () => client.get('/fails'),
        /Scrappa API error \(500\): Server Error/,
    );
});
