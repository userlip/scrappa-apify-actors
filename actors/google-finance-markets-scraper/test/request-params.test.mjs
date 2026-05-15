import assert from 'node:assert/strict';
import test from 'node:test';

import { buildGoogleFinanceMarketsParams, describeGoogleFinanceMarketsRequest } from '../dist/request-params.js';

test('builds default params for a gainers trend request', () => {
    assert.deepEqual(
        buildGoogleFinanceMarketsParams({
            hl: 'EN',
            gl: 'US',
        }),
        {
            trend: 'gainers',
            hl: 'en',
            gl: 'us',
        },
    );
});

test('builds params for trend and index market filters', () => {
    assert.deepEqual(
        buildGoogleFinanceMarketsParams({
            trend: ' INDEXES ',
            index_market: ' Americas ',
            hl: 'en',
            gl: 'us',
        }),
        {
            trend: 'indexes',
            index_market: 'americas',
            hl: 'en',
            gl: 'us',
        },
    );
});

test('rejects invalid trends and index markets', () => {
    assert.throws(
        () => buildGoogleFinanceMarketsParams({ trend: 'hot-stocks' }),
        /trend must be one of:/,
    );
    assert.throws(
        () => buildGoogleFinanceMarketsParams({ index_market: 'africa' }),
        /index_market must be one of:/,
    );
    assert.throws(
        () => buildGoogleFinanceMarketsParams({ trend: 'gainers', index_market: 'americas' }),
        /index_market can only be used when trend is indexes/,
    );
    assert.throws(
        () => buildGoogleFinanceMarketsParams({ index_market: 'americas' }),
        /index_market can only be used when trend is indexes/,
    );
});

test('rejects invalid localization controls', () => {
    assert.throws(
        () => buildGoogleFinanceMarketsParams({ hl: 'english' }),
        /hl must be a valid language code/,
    );
    assert.throws(
        () => buildGoogleFinanceMarketsParams({ gl: 'usa' }),
        /gl must be a two-letter country code/,
    );
});

test('describes requests for logs', () => {
    assert.equal(
        describeGoogleFinanceMarketsRequest({
            trend: 'indexes',
            index_market: 'asia-pacific',
            hl: 'en',
            gl: 'us',
        }),
        'trend=indexes (index_market=asia-pacific, hl=en, gl=us)',
    );

    assert.equal(describeGoogleFinanceMarketsRequest({ trend: 'gainers' }), 'trend=gainers');
});
