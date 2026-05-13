import assert from 'node:assert/strict';
import test from 'node:test';

import { buildGoogleFinanceHistoricalPricesParams, describeGoogleFinanceHistoricalPricesRequest } from '../dist/request-params.js';

test('builds params for a preset historical prices request', () => {
    assert.deepEqual(
        buildGoogleFinanceHistoricalPricesParams({
            symbol: ' aapl ',
            exchange: ' nasdaq ',
            range: '6',
            interval: 'WEEKLY',
            hl: 'EN',
            gl: 'US',
        }),
        {
            symbol: 'AAPL',
            exchange: 'NASDAQ',
            range: 6,
            interval: 'weekly',
            hl: 'en',
            gl: 'us',
        },
    );
});

test('builds params for a custom date range request', () => {
    assert.deepEqual(
        buildGoogleFinanceHistoricalPricesParams({
            symbol: 'MSFT',
            exchange: 'NASDAQ',
            start_date: '2024-01-01',
            end_date: '2024-01-31',
            interval: 'daily',
        }),
        {
            symbol: 'MSFT',
            exchange: 'NASDAQ',
            start_date: '2024-01-01',
            end_date: '2024-01-31',
            interval: 'daily',
        },
    );
});

test('requires a non-empty symbol', () => {
    assert.throws(
        () => buildGoogleFinanceHistoricalPricesParams({ symbol: '   ' }),
        /symbol is required/,
    );
});

test('rejects invalid symbol and localization controls', () => {
    assert.throws(
        () => buildGoogleFinanceHistoricalPricesParams({ symbol: 'BRK B' }),
        /symbol cannot contain spaces/,
    );
    assert.throws(
        () => buildGoogleFinanceHistoricalPricesParams({ symbol: 'AAPL', hl: 'english' }),
        /hl must be a valid language code/,
    );
    assert.throws(
        () => buildGoogleFinanceHistoricalPricesParams({ symbol: 'AAPL', gl: 'country-code' }),
        /gl must be 10 characters or fewer/,
    );
});

test('rejects invalid ranges and intervals', () => {
    assert.throws(
        () => buildGoogleFinanceHistoricalPricesParams({ symbol: 'AAPL', range: 9 }),
        /range must be between 1 and 8/,
    );
    assert.throws(
        () => buildGoogleFinanceHistoricalPricesParams({ symbol: 'AAPL', interval: 'hourly' }),
        /interval must be one of: daily, weekly, monthly/,
    );
});

test('rejects conflicting range and date controls', () => {
    assert.throws(
        () => buildGoogleFinanceHistoricalPricesParams({
            symbol: 'AAPL',
            range: 6,
            start_date: '2024-01-01',
            end_date: '2024-01-31',
        }),
        /Cannot use both range and start_date\/end_date/,
    );
    assert.throws(
        () => buildGoogleFinanceHistoricalPricesParams({ symbol: 'AAPL', start_date: '2024-01-01' }),
        /start_date and end_date must be provided together/,
    );
    assert.throws(
        () => buildGoogleFinanceHistoricalPricesParams({
            symbol: 'AAPL',
            start_date: '2024-02-01',
            end_date: '2024-01-01',
        }),
        /end_date must be on or after start_date/,
    );
});

test('describes historical price requests for logs', () => {
    assert.equal(
        describeGoogleFinanceHistoricalPricesRequest({
            symbol: 'AAPL',
            exchange: 'NASDAQ',
            range: 6,
            interval: 'daily',
            hl: 'en',
            gl: 'us',
        }),
        'AAPL:NASDAQ (range=6, interval=daily, hl=en, gl=us)',
    );
});
