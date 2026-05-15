import assert from 'node:assert/strict';
import test from 'node:test';

import { buildMarketsDatasetItems, buildMarketsResultCounts } from '../dist/response-utils.js';

test('builds dataset items from overview sections', () => {
    const response = {
        markets: {
            us: [
                {
                    stock: '.DJI:INDEXDJX',
                    name: 'Dow Jones Industrial Average',
                    symbol: '.DJI',
                    exchange: 'INDEXDJX',
                    price: 42515.09,
                    price_movement: { direction: 'Down', value: -125.69, percentage: -0.3 },
                },
            ],
            currencies: [
                {
                    stock: 'EUR-USD',
                    name: 'EUR / USD',
                    price: '1.0457',
                    from_currency: 'EUR',
                    to_currency: 'USD',
                    price_movement: { direction: 'Up', value: '0.0012', percentage: '0.11' },
                },
            ],
        },
    };

    const items = buildMarketsDatasetItems(response, { hl: 'en', gl: 'us' });

    assert.equal(items.length, 2);
    assert.equal(items[0].section, 'us');
    assert.equal(items[0].stock, '.DJI:INDEXDJX');
    assert.equal(items[0].price_movement_direction, 'Down');
    assert.equal(items[1].section, 'currencies');
    assert.equal(items[1].price, 1.0457);
    assert.equal(items[1].from_currency, 'EUR');
    assert.deepEqual(items[0].result_counts.market_rows, 2);
});

test('builds dataset items from trend groups and news results', () => {
    const response = {
        market_trends: [
            {
                title: 'Americas',
                results: [
                    {
                        stock: 'AAPL:NASDAQ',
                        link: 'https://www.google.com/finance/quote/AAPL:NASDAQ',
                        name: 'Apple Inc',
                        symbol: 'AAPL',
                        exchange: 'NASDAQ',
                        extracted_price: 189.98,
                        currency: 'USD',
                        price_movement: { direction: 'Up', value: 3.25, percentage: 1.74 },
                    },
                ],
            },
        ],
        news_results: [
            {
                title: 'Markets climb',
                link: 'https://example.com/news',
                source: 'Example Finance',
                date: '2026-05-15 13:30:00',
                snippet: 'Stocks moved higher.',
                thumbnail: 'https://example.com/thumb.jpg',
            },
        ],
    };

    const items = buildMarketsDatasetItems(response, { trend: 'gainers', hl: 'en', gl: 'us' });

    assert.equal(items.length, 2);
    assert.equal(items[0].item_type, 'market_row');
    assert.equal(items[0].section, 'Americas');
    assert.equal(items[0].trend_group, 'Americas');
    assert.equal(items[0].trend, 'gainers');
    assert.equal(items[0].price, 189.98);
    assert.equal(items[1].item_type, 'news_result');
    assert.equal(items[1].section, 'finance-news');
    assert.equal(items[1].title, 'Markets climb');
});

test('counts overview, trend, and news rows', () => {
    assert.deepEqual(
        buildMarketsResultCounts({
            markets: { us: [{ stock: 'SPY:NYSEARCA' }], futures: [{ stock: 'YMW00:CBOT' }] },
            market_trends: [{ results: [{ stock: 'AAPL:NASDAQ' }, { stock: 'MSFT:NASDAQ' }] }],
            news_results: [{ title: 'News' }],
        }),
        {
            market_rows: 4,
            trend_groups: 1,
            trend_rows: 2,
            news_results: 1,
            us: 1,
            europe: 0,
            asia: 0,
            currencies: 0,
            crypto: 0,
            futures: 1,
        },
    );
});
