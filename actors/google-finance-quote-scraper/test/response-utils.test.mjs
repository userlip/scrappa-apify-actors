import assert from 'node:assert/strict';
import test from 'node:test';

import { buildQuoteDatasetItem } from '../dist/response-utils.js';

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
        { quote: { summary: { price: 125 } } },
        { symbol: 'VOO' },
    );

    assert.equal(item.symbol, 'VOO');
    assert.equal(item.current_price, 125);
    assert.deepEqual(item.financials, []);
    assert.deepEqual(item.news, []);
    assert.deepEqual(item.related_tickers, []);
});
