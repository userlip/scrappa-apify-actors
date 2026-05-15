import assert from 'node:assert/strict';
import test from 'node:test';

import { buildQuoteDatasetItem, hasMeaningfulQuoteData } from '../dist/response-utils.js';

test('builds one dataset item with nested quote data', () => {
    const item = buildQuoteDatasetItem(
        {
            quote: {
                summary: {
                    symbol: 'AAPL',
                    exchange: 'NASDAQ',
                    name: 'Apple Inc',
                    current_price: '198.53',
                    currency: 'USD',
                    price_change: '1.18',
                    percent_change: '0.6',
                    market_status: 'Closed',
                },
                key_stats: {
                    'Market cap': '2.96T USD',
                },
                about: {
                    description: 'Apple Inc. profile',
                },
                financials: [{ title: 'Revenue' }],
                news: [{ title: 'Apple news' }],
                discover_more: [{ title: 'Related', items: [{ symbol: 'MSFT' }] }],
            },
        },
        {
            symbol: 'AAPL',
            exchange: 'NASDAQ',
            period_type: 'quarterly',
            hl: 'en',
            gl: 'us',
        },
    );

    assert.equal(item.symbol, 'AAPL');
    assert.equal(item.exchange, 'NASDAQ');
    assert.equal(item.current_price, 198.53);
    assert.deepEqual(item.key_stats, { 'Market cap': '2.96T USD' });
    assert.deepEqual(item.related_tickers, [{ symbol: 'MSFT' }]);
    assert.deepEqual(item.result_counts, {
        financials: 1,
        news: 1,
        discover_more: 1,
        related_tickers: 1,
    });
});

test('falls back to request symbol and empty arrays when sections are missing', () => {
    const item = buildQuoteDatasetItem(
        { quote: { summary: { price: '   ', last_price: 125 } } },
        { symbol: 'VOO' },
    );

    assert.equal(item.symbol, 'VOO');
    assert.equal(item.current_price, 125);
    assert.deepEqual(item.financials, []);
    assert.deepEqual(item.news, []);
    assert.deepEqual(item.related_tickers, []);
});

test('does not treat discover_more section wrappers as related tickers', () => {
    const discoverMore = [
        { title: 'You may be interested in', description: 'Popular market lists' },
        { title: 'Indexes', groups: [{ symbol: '.INX' }] },
    ];

    const item = buildQuoteDatasetItem(
        { quote: { discover_more: discoverMore } },
        { symbol: 'AAPL' },
    );

    assert.deepEqual(item.discover_more, discoverMore);
    assert.deepEqual(item.related_tickers, []);
    assert.equal(item.result_counts.related_tickers, 0);
    assert.equal(item.result_counts.discover_more, 2);
});

test('detects whether a quote response has usable quote data', () => {
    assert.equal(hasMeaningfulQuoteData({}), false);
    assert.equal(hasMeaningfulQuoteData({ quote: { summary: {}, key_stats: {}, about: {} } }), false);
    assert.equal(hasMeaningfulQuoteData({ quote: { summary: { name: 'Finance', symbol: 'MSFT', exchange: 'NASDAQ' } } }), false);
    assert.equal(hasMeaningfulQuoteData({ quote: { key_stats: { stats: [], tags: [], climate_change: {} } } }), false);
    assert.equal(hasMeaningfulQuoteData({ quote: { key_stats: { stats: [{}], tags: [{ text: '' }], climate_change: { score: null } } } }), false);
    assert.equal(hasMeaningfulQuoteData({ quote: { summary: { current_price: 0 } } }), true);
    assert.equal(hasMeaningfulQuoteData({ quote: { summary: { current_price: '423.85' } } }), true);
    assert.equal(hasMeaningfulQuoteData({ quote: { summary: { change: '0' } } }), true);
    assert.equal(hasMeaningfulQuoteData({ quote: { summary: { change: '-1.27' } } }), true);
    assert.equal(hasMeaningfulQuoteData({ quote: { summary: { market_state: 'Closed' } } }), true);
    assert.equal(hasMeaningfulQuoteData({ quote: { about: { description: 'Microsoft Corporation profile' } } }), true);
    assert.equal(hasMeaningfulQuoteData({ quote: { key_stats: { market_cap: '3.15T USD' } } }), true);
    assert.equal(hasMeaningfulQuoteData({ quote: { key_stats: { stats: [{ label: 'Avg volume', value: '21M' }] } } }), true);
    assert.equal(hasMeaningfulQuoteData({ quote: { key_stats: { is_index: false } } }), true);
    assert.equal(hasMeaningfulQuoteData({ quote: { financials: [{ title: 'Income Statement' }] } }), true);
});
