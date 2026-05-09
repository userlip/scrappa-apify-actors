import assert from 'node:assert/strict';
import test from 'node:test';

import { buildGoogleFinanceQuoteParams, describeGoogleFinanceQuoteRequest } from '../dist/request-params.js';

test('builds params for a complete quote request', () => {
    assert.deepEqual(
        buildGoogleFinanceQuoteParams({
            symbol: ' aapl ',
            exchange: ' nasdaq ',
            period_type: 'ANNUAL',
            hl: 'EN',
            gl: 'US',
        }),
        {
            symbol: 'AAPL',
            exchange: 'NASDAQ',
            period_type: 'annual',
            hl: 'en',
            gl: 'us',
        },
    );
});

test('requires a non-empty symbol', () => {
    assert.throws(
        () => buildGoogleFinanceQuoteParams({ symbol: '   ' }),
        /symbol is required/,
    );
});

test('rejects invalid symbol and localization controls', () => {
    assert.throws(
        () => buildGoogleFinanceQuoteParams({ symbol: 'BRK B' }),
        /symbol cannot contain spaces/,
    );
    assert.throws(
        () => buildGoogleFinanceQuoteParams({ symbol: 'AAPL', hl: 'english' }),
        /hl must be a valid language code/,
    );
    assert.throws(
        () => buildGoogleFinanceQuoteParams({ symbol: 'AAPL', gl: 'usa' }),
        /gl must be 2 characters or fewer/,
    );
});

test('rejects invalid period type', () => {
    assert.throws(
        () => buildGoogleFinanceQuoteParams({ symbol: 'AAPL', period_type: 'monthly' }),
        /period_type must be one of: quarterly, annual/,
    );
});

test('describes quote requests for logs', () => {
    assert.equal(
        describeGoogleFinanceQuoteRequest({
            symbol: 'AAPL',
            exchange: 'NASDAQ',
            period_type: 'quarterly',
            hl: 'en',
            gl: 'us',
        }),
        'AAPL:NASDAQ (period_type=quarterly, hl=en, gl=us)',
    );
});
