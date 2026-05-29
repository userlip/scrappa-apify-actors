import assert from 'node:assert/strict';
import test from 'node:test';

import { buildGoogleFinanceIntradayRequests, describeGoogleFinanceIntradayRequest } from '../dist/request-params.js';

test('builds params for a batch of intraday symbol requests', () => {
    assert.deepEqual(
        buildGoogleFinanceIntradayRequests({
            symbols: [
                { symbol: ' aapl ', exchange: ' nasdaq ' },
                { symbol: 'msft' },
            ],
            hl: 'EN',
            gl: 'US',
        }),
        [
            {
                symbol: 'AAPL',
                exchange: 'NASDAQ',
                hl: 'en',
                gl: 'us',
            },
            {
                symbol: 'MSFT',
                hl: 'en',
                gl: 'us',
            },
        ],
    );
});

test('requires at least one symbol object', () => {
    assert.throws(
        () => buildGoogleFinanceIntradayRequests({}),
        /symbols must be an array/,
    );
    assert.throws(
        () => buildGoogleFinanceIntradayRequests({ symbols: [] }),
        /At least one symbol is required/,
    );
    assert.throws(
        () => buildGoogleFinanceIntradayRequests({ symbols: ['AAPL'] }),
        /symbols\[0\] must be an object/,
    );
});

test('rejects invalid symbol and localization controls', () => {
    assert.throws(
        () => buildGoogleFinanceIntradayRequests({ symbols: [{ symbol: '   ' }] }),
        /symbols\[0\]\.symbol is required/,
    );
    assert.throws(
        () => buildGoogleFinanceIntradayRequests({ symbols: [{ symbol: 'BRK B' }] }),
        /symbols\[0\]\.symbol cannot contain spaces/,
    );
    assert.throws(
        () => buildGoogleFinanceIntradayRequests({ symbols: [{ symbol: 'AAPL' }], hl: 'english' }),
        /hl must be a valid language code/,
    );
    assert.throws(
        () => buildGoogleFinanceIntradayRequests({ symbols: [{ symbol: 'AAPL' }], gl: 'usa' }),
        /gl must be a two-letter country code/,
    );
});

test('describes intraday requests for logs', () => {
    assert.equal(
        describeGoogleFinanceIntradayRequest({
            symbol: 'AAPL',
            exchange: 'NASDAQ',
            hl: 'en',
            gl: 'us',
        }),
        'AAPL:NASDAQ (hl=en, gl=us)',
    );
});
