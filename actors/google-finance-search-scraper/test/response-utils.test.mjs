import assert from 'node:assert/strict';
import test from 'node:test';

import { buildSearchDatasetItems, countSearchResults } from '../dist/response-utils.js';

test('builds dataset items from Google Finance search results', () => {
    const response = {
        results: [
            {
                stock: 'AAPL:NASDAQ',
                name: 'Apple Inc',
                symbol: 'AAPL',
                exchange: 'NASDAQ',
                type: 'Stock',
                currency: 'USD',
                price: '176.85',
                price_movement: { value: '2.34', percentage: '1.33' },
            },
        ],
    };

    const items = buildSearchDatasetItems(response, { q: 'AAPL', hl: 'en', gl: 'us' });

    assert.equal(items.length, 1);
    assert.equal(items[0].query, 'AAPL');
    assert.equal(items[0].position, 1);
    assert.equal(items[0].name, 'Apple Inc');
    assert.equal(items[0].symbol, 'AAPL');
    assert.equal(items[0].exchange, 'NASDAQ');
    assert.equal(items[0].price, 176.85);
    assert.equal(items[0].price_change, 2.34);
    assert.equal(items[0].percent_change, 1.33);
    assert.equal(items[0].google_finance_url, 'https://www.google.com/finance/quote/AAPL%3ANASDAQ');
    assert.deepEqual(items[0].raw_result, response.results[0]);
});

test('extracts results from alternate response wrappers', () => {
    const response = {
        data: {
            search_results: [
                {
                    title: 'Tesla Inc',
                    symbol: 'TSLA',
                    exchange: 'NASDAQ',
                    link: 'https://www.google.com/finance/quote/TSLA:NASDAQ',
                },
            ],
        },
    };

    const items = buildSearchDatasetItems(response, { q: 'Tesla' });

    assert.equal(countSearchResults(response), 1);
    assert.equal(items[0].name, 'Tesla Inc');
    assert.equal(items[0].link, 'https://www.google.com/finance/quote/TSLA:NASDAQ');
});

test('returns no dataset items for empty search responses', () => {
    assert.equal(countSearchResults({ results: [] }), 0);
    assert.deepEqual(buildSearchDatasetItems({}, { q: 'not-a-match' }), []);
});
