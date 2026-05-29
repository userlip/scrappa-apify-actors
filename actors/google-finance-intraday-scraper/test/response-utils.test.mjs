import assert from 'node:assert/strict';
import test from 'node:test';

import { buildIntradayPricePointDatasetItems } from '../dist/response-utils.js';

test('builds dataset items from intraday graph points', () => {
    const items = buildIntradayPricePointDatasetItems(
        {
            symbol: 'AAPL',
            exchange: 'NASDAQ',
            currency: 'USD',
            graph: [
                {
                    date: 'Jun 16 2025, 09:30 AM UTC-04:00',
                    price: '198.42',
                    change: '1.25',
                    percent_change: '0.63',
                    volume: '3,482,103',
                },
            ],
        },
        {
            symbol: 'AAPL',
            exchange: 'NASDAQ',
            hl: 'en',
            gl: 'us',
        },
    );

    assert.equal(items.length, 1);
    assert.equal(items[0].position, 1);
    assert.equal(items[0].date, 'Jun 16 2025, 09:30 AM UTC-04:00');
    assert.equal(items[0].date_iso, '2025-06-16T13:30:00.000Z');
    assert.equal(items[0].price, 198.42);
    assert.equal(items[0].change, 1.25);
    assert.equal(items[0].percent_change, 0.63);
    assert.equal(items[0].volume, 3482103);
    assert.equal(items[0].currency, 'USD');
    assert.deepEqual(items[0].result_counts, { graph: 1 });
});

test('falls back to request symbol and returns no rows for missing graph', () => {
    const items = buildIntradayPricePointDatasetItems(
        { currency: 'USD' },
        { symbol: 'VOO', exchange: 'NYSEARCA' },
    );

    assert.deepEqual(items, []);
});
