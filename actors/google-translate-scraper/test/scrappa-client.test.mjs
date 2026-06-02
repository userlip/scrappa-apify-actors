import assert from 'node:assert/strict';
import test from 'node:test';

const sharedModule = process.env.TEST_SOURCE === 'src'
    ? '../src/shared/index.ts'
    : '../dist/shared/index.js';
const {
    ScrappaClient,
    ScrappaHttpError,
    ScrappaNetworkError,
    getRetryDelayMs,
} = await import(sharedModule);

test('ScrappaClient builds the Google Translate request with actor user agent', async () => {
    const originalFetch = globalThis.fetch;
    let capturedUrl;
    let capturedOptions;

    globalThis.fetch = async (url, options) => {
        capturedUrl = url;
        capturedOptions = options;
        return new Response(JSON.stringify({ translated_text: 'Hallo' }), { status: 200 });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        const result = await client.get('/google-translate', {
            text: 'Good morning',
            source: 'en',
            target: 'de',
        });

        assert.deepEqual(result, { translated_text: 'Hallo' });
        const url = new URL(capturedUrl);
        assert.equal(url.pathname, '/api/google-translate');
        assert.equal(url.searchParams.get('text'), 'Good morning');
        assert.equal(url.searchParams.get('source'), 'en');
        assert.equal(url.searchParams.get('target'), 'de');
        assert.equal(url.searchParams.has('append'), false);
        assert.equal(url.searchParams.has('html'), false);
        assert.equal(capturedOptions.headers['X-API-Key'], 'test-key');
        assert.equal(capturedOptions.headers['User-Agent'], 'thescrappa-google-translate-scraper/1.0');
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('ScrappaClient surfaces JSON error fields from Scrappa', async () => {
    const originalFetch = globalThis.fetch;

    globalThis.fetch = async () => new Response(
        JSON.stringify({ error: 'Translation service temporarily unavailable. Please retry.' }),
        { status: 503 },
    );

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        await assert.rejects(
            () => client.get('/google-translate', { text: 'Hello', source: 'en', target: 'de' }, { attempts: 1 }),
            (error) => error instanceof ScrappaHttpError
                && error.status === 503
                && error.details === 'Translation service temporarily unavailable. Please retry.',
        );
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('ScrappaClient exposes persistent network failures as ScrappaNetworkError', async () => {
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
            () => client.get('/google-translate', { text: 'Hello', source: 'en', target: 'de' }, { attempts: 1 }),
            (error) => error instanceof ScrappaNetworkError && error.message === 'Scrappa API network error: fetch failed',
        );
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('ScrappaClient retries transient Scrappa errors before returning success', async () => {
    const originalFetch = globalThis.fetch;
    const delays = [];
    let calls = 0;

    globalThis.fetch = async () => {
        calls += 1;
        return calls === 1
            ? new Response(JSON.stringify({ error: 'Temporary upstream failure.' }), { status: 503 })
            : new Response(JSON.stringify({ translated_text: 'Hallo' }), { status: 200 });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        const result = await client.get('/google-translate', { text: 'Hello', source: 'en', target: 'de' }, {
            attempts: 2,
            retryDelayMs(failedAttempt) {
                delays.push(failedAttempt);
                return 0;
            },
        });

        assert.deepEqual(result, { translated_text: 'Hallo' });
        assert.equal(calls, 2);
        assert.deepEqual(delays, [1]);
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('ScrappaClient does not retry non-retryable Scrappa errors', async () => {
    const originalFetch = globalThis.fetch;
    let calls = 0;

    globalThis.fetch = async () => {
        calls += 1;
        return new Response(JSON.stringify({ message: 'Invalid target language.' }), { status: 400 });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        await assert.rejects(
            () => client.get('/google-translate', { text: 'Hello', source: 'en', target: 'invalid' }, {
                attempts: 3,
                retryDelayMs() {
                    throw new Error('retry delay should not be used');
                },
            }),
            (error) => error instanceof ScrappaHttpError
                && error.status === 400
                && error.details === 'Invalid target language.',
        );
        assert.equal(calls, 1);
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('ScrappaClient surfaces the last transient error after retries are exhausted', async () => {
    const originalFetch = globalThis.fetch;
    const delays = [];
    let calls = 0;

    globalThis.fetch = async () => {
        calls += 1;
        return new Response(JSON.stringify({ error: `Temporary upstream failure ${calls}.` }), { status: 503 });
    };

    try {
        const client = new ScrappaClient({
            apiKey: 'test-key',
            baseUrl: 'https://example.test/api',
        });

        await assert.rejects(
            () => client.get('/google-translate', { text: 'Hello', source: 'en', target: 'de' }, {
                attempts: 3,
                retryDelayMs(failedAttempt) {
                    delays.push(failedAttempt);
                    return 0;
                },
            }),
            (error) => error instanceof ScrappaHttpError
                && error.status === 503
                && error.details === 'Temporary upstream failure 3.',
        );
        assert.equal(calls, 3);
        assert.deepEqual(delays, [1, 2]);
    } finally {
        globalThis.fetch = originalFetch;
    }
});

test('getRetryDelayMs applies exponential backoff with deterministic jitter', () => {
    assert.equal(getRetryDelayMs(1, 250), 2250);
    assert.equal(getRetryDelayMs(4, 250), 10000);
});
