import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildQuoteResponseFromSearchResult,
    fetchQuoteWithFallback,
    fetchSearchQuoteFallback,
    shouldRetryBaseQuote,
} from '../dist/quote-fetch.js';
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

test('propagates fallback Scrappa 5xx after one degraded retry pass', async () => {
    const originalWarn = console.warn;
    const primaryError = new ScrappaHttpError(500, 'Internal Server Error');
    const fallbackError = new ScrappaHttpError(503, 'Service unavailable');
    const { client, calls } = createClientStub([primaryError, fallbackError]);

    console.warn = () => {};

    try {
        await assert.rejects(
            () => fetchQuoteWithFallback(client, { symbol: 'MSFT', period_type: 'quarterly' }, 3),
            fallbackError,
        );
        assert.equal(calls.length, 2);
        assert.deepEqual(calls[0].params, { symbol: 'MSFT', period_type: 'quarterly' });
        assert.deepEqual(calls[1].params, { symbol: 'MSFT' });
    } finally {
        console.warn = originalWarn;
    }
});

test('classifies only Scrappa 5xx errors with period_type for base quote fallback', () => {
    assert.equal(shouldRetryBaseQuote(new ScrappaHttpError(500, 'Internal Server Error'), { period_type: 'quarterly' }), true);
    assert.equal(shouldRetryBaseQuote(new ScrappaHttpError(503, 'Service unavailable'), { period_type: 'annual' }), true);
    assert.equal(shouldRetryBaseQuote(new ScrappaHttpError(429, 'Rate limited'), { period_type: 'quarterly' }), false);
    assert.equal(shouldRetryBaseQuote(new ScrappaHttpError(500, 'Internal Server Error'), { symbol: 'MSFT' }), false);
    assert.equal(shouldRetryBaseQuote(new Error('Scrappa API error (500): Internal Server Error'), { period_type: 'quarterly' }), false);
});

test('builds a quote-shaped response from an exact search result fallback', () => {
    const response = buildQuoteResponseFromSearchResult(
        {
            symbol: 'MSFT',
            exchange: 'NASDAQ',
            name: 'Microsoft Corp',
            currency: 'USD',
            current_price: 409.43,
            price_change: 4.22,
            percent_change: 1.04,
            previous_close: 405.21,
            country: 'US',
        },
        { symbol: 'MSFT', exchange: 'NASDAQ' },
    );

    assert.deepEqual(response, {
        quote: {
            summary: {
                name: 'Microsoft Corp',
                symbol: 'MSFT',
                exchange: 'NASDAQ',
                current_price: 409.43,
                price_change: 4.22,
                percent_change: 1.04,
                currency: 'USD',
                country: 'US',
            },
            key_stats: {
                previous_close: 405.21,
            },
            about: {},
            financials: [],
            news: [],
            discover_more: [],
        },
    });
});

test('returns search fallback for the requested symbol and exchange', async () => {
    const { client, calls } = createClientStub([
        {
            results: [
                { symbol: 'MSFT', exchange: 'BMV', current_price: 7055 },
                { symbol: 'MSFT', exchange: 'NASDAQ', current_price: 409.43, currency: 'USD' },
            ],
        },
    ]);

    const result = await fetchSearchQuoteFallback(
        client,
        { symbol: 'MSFT', exchange: 'NASDAQ', period_type: 'quarterly', hl: 'en', gl: 'us' },
        3,
    );

    assert.equal(result.response.quote.summary.current_price, 409.43);
    assert.deepEqual(result.fallback, {
        reason: 'scrappa_quote_empty_search_result',
        omitted_params: ['period_type'],
        primary_error: 'Scrappa quote response did not contain usable price, key stats, profile, financials, news, or related ticker data.',
        source_endpoint: '/google-finance/search',
        unavailable_sections: ['about', 'financials', 'news', 'discover_more'],
    });
    assert.deepEqual(calls, [
        {
            endpoint: '/google-finance/search',
            params: { q: 'MSFT', hl: 'en', gl: 'us' },
            options: { attempts: 3 },
        },
    ]);
});

test('returns null when search fallback cannot match the requested exchange', async () => {
    const { client } = createClientStub([
        {
            results: [
                { symbol: 'MSFT', exchange: 'BMV', current_price: 7055 },
            ],
        },
    ]);

    const result = await fetchSearchQuoteFallback(client, { symbol: 'MSFT', exchange: 'NASDAQ' }, 3);

    assert.equal(result, null);
});
