import assert from 'node:assert/strict';
import test from 'node:test';

import {
    ScrappaClient,
    ScrappaHttpError,
    ScrappaTimeoutError,
    getRetryDelayMs,
    isRetryableScrappaError,
} from '../dist/shared/index.js';

test('builds GET requests with filtered params and actor user agent', async () => {
    const originalFetch = globalThis.fetch;
    let capturedUrl;
    let capturedOptions;

    globalThis.fetch = async (url, options) => {
        capturedUrl = url;
        capturedOptions = options;
        return new Response(JSON.stringify({ ok: true }), { status: 200 });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        const result = await client.get('/google-finance/quote', {
            symbol: 'AAPL',
            exchange: 'NASDAQ',
            debug: false,
            use_cache: true,
            empty: '',
            missing: undefined,
            absent: null,
        });

        assert.deepEqual(result, { ok: true });
        const url = new URL(capturedUrl);
        assert.equal(url.pathname, '/api/google-finance/quote');
        assert.equal(url.searchParams.get('symbol'), 'AAPL');
        assert.equal(url.searchParams.get('exchange'), 'NASDAQ');
        assert.equal(url.searchParams.get('use_cache'), '1');
        assert.equal(url.searchParams.has('debug'), false);
        assert.equal(url.searchParams.has('empty'), false);
        assert.equal(capturedOptions.headers['X-API-Key'], 'test-key');
        assert.equal(capturedOptions.headers['User-Agent'], 'thescrappa-google-finance-quote-scraper/1.0');
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('parses Scrappa JSON error messages and validation details', async () => {
    const originalFetch = globalThis.fetch;

    globalThis.fetch = async () => new Response(
        JSON.stringify({
            message: 'Invalid request',
            errors: {
                symbol: ['The stock symbol is required.'],
            },
        }),
        { status: 422, statusText: 'Unprocessable Entity' },
    );

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        await assert.rejects(
            () => client.get('/google-finance/quote'),
            (error) => {
                assert.equal(error instanceof ScrappaHttpError, true);
                assert.equal(error.status, 422);
                assert.equal(error.details, 'Invalid request - symbol: The stock symbol is required.');
                assert.match(error.message, /Scrappa API error \(422\): Invalid request - symbol: The stock symbol is required\./);
                return true;
            },
        );
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('preserves timeout classification when reading an error response aborts', async () => {
    const originalFetch = globalThis.fetch;

    globalThis.fetch = async () => ({
        ok: false,
        status: 500,
        statusText: 'Server Error',
        text: async () => {
            const error = new Error('aborted');
            error.name = 'AbortError';
            throw error;
        },
    });

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
            timeoutMs: 1000,
        });

        await assert.rejects(
            () => client.get('/google-finance/quote'),
            ScrappaTimeoutError,
        );
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('classifies retryable Scrappa errors and timeout errors', () => {
    assert.equal(isRetryableScrappaError(new ScrappaTimeoutError(1000)), true);
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(500, 'Internal Server Error')), true);
    assert.equal(isRetryableScrappaError(new ScrappaHttpError(422, 'Invalid request')), false);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (429): Rate limited')), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (503): Service unavailable')), true);
    assert.equal(isRetryableScrappaError(new Error('Scrappa API error (422): Invalid request')), false);
    assert.equal(isRetryableScrappaError('Scrappa API error (503): Service unavailable'), false);
});

test('calculates deterministic retry delay with bounded backoff', () => {
    assert.equal(getRetryDelayMs(1, 0), 2000);
    assert.equal(getRetryDelayMs(2, 500), 4500);
    assert.equal(getRetryDelayMs(20, 0), 10000);
});
