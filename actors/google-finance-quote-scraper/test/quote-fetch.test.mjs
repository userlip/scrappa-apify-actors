import assert from 'node:assert/strict';
import test from 'node:test';

import { fetchQuoteWithFallback, shouldRetryBaseQuote } from '../dist/quote-fetch.js';
import { ScrappaHttpError } from '../dist/shared/index.js';

function createClientStub(results) {
    const calls = [];

    return {
        calls,
        client: {
            async get(endpoint, params, options) {
                calls.push({ endpoint, params, options });
                const result = results.shift();

                if (result instanceof Error) {
                    throw result;
                }

                return result;
            },
        },
    };
}

test('returns primary quote response without fallback metadata when the primary request succeeds', async () => {
    const primaryResponse = { quote: { summary: { symbol: 'MSFT' } } };
    const { client, calls } = createClientStub([primaryResponse]);

    const result = await fetchQuoteWithFallback(
        client,
        { symbol: 'MSFT', exchange: 'NASDAQ', period_type: 'quarterly' },
        3,
    );

    assert.deepEqual(result, { response: primaryResponse });
    assert.equal(calls.length, 1);
    assert.deepEqual(calls[0], {
        endpoint: '/google-finance/quote',
        params: { symbol: 'MSFT', exchange: 'NASDAQ', period_type: 'quarterly' },
        options: { attempts: 3 },
    });
});

test('retries base quote without period_type after primary Scrappa 5xx', async () => {
    const originalWarn = console.warn;
    const fallbackResponse = { quote: { summary: { symbol: 'MSFT', exchange: 'NASDAQ' } } };
    const primaryError = new ScrappaHttpError(500, 'Internal Server Error');
    const { client, calls } = createClientStub([primaryError, fallbackResponse]);

    console.warn = () => {};

    try {
        const result = await fetchQuoteWithFallback(
            client,
            { symbol: 'MSFT', exchange: 'NASDAQ', period_type: 'quarterly', hl: 'en' },
            3,
        );

        assert.deepEqual(result, {
            response: fallbackResponse,
            fallback: {
                reason: 'scrappa_5xx_after_financial_period_request',
                omitted_params: ['period_type'],
                primary_error: 'Scrappa API error (500): Internal Server Error',
            },
        });
        assert.equal(calls.length, 2);
        assert.deepEqual(calls[0].params, { symbol: 'MSFT', exchange: 'NASDAQ', period_type: 'quarterly', hl: 'en' });
        assert.deepEqual(calls[1].params, { symbol: 'MSFT', exchange: 'NASDAQ', hl: 'en' });
    } finally {
        console.warn = originalWarn;
    }
});

test('propagates Scrappa 5xx when the original request has no period_type', async () => {
    const primaryError = new ScrappaHttpError(500, 'Internal Server Error');
    const { client, calls } = createClientStub([primaryError]);

    await assert.rejects(
        () => fetchQuoteWithFallback(client, { symbol: 'MSFT', exchange: 'NASDAQ' }, 3),
        primaryError,
    );
    assert.equal(calls.length, 1);
});

test('propagates non-retryable Scrappa errors without fallback', async () => {
    const validationError = new ScrappaHttpError(422, 'Invalid request');
    const { client, calls } = createClientStub([validationError]);

    await assert.rejects(
        () => fetchQuoteWithFallback(client, { symbol: 'MSFT', period_type: 'quarterly' }, 3),
        validationError,
    );
    assert.equal(calls.length, 1);
});

test('classifies only Scrappa 5xx errors with period_type for base quote fallback', () => {
    assert.equal(shouldRetryBaseQuote(new ScrappaHttpError(500, 'Internal Server Error'), { period_type: 'quarterly' }), true);
    assert.equal(shouldRetryBaseQuote(new ScrappaHttpError(503, 'Service unavailable'), { period_type: 'annual' }), true);
    assert.equal(shouldRetryBaseQuote(new ScrappaHttpError(429, 'Rate limited'), { period_type: 'quarterly' }), false);
    assert.equal(shouldRetryBaseQuote(new ScrappaHttpError(500, 'Internal Server Error'), { symbol: 'MSFT' }), false);
    assert.equal(shouldRetryBaseQuote(new Error('Scrappa API error (500): Internal Server Error'), { period_type: 'quarterly' }), false);
});
