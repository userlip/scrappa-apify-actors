import assert from 'node:assert/strict';
import test from 'node:test';

import { buildHistoricalPriceDatasetItems } from '../dist/response-utils.js';

test('builds dataset items from historical price points', () => {
    const items = buildHistoricalPriceDatasetItems(
        {
            symbol: 'AAPL',
            exchange: 'NASDAQ',
            currency: 'USD',
            previous_close: '275.50',
            prices: [
                {
                    date: 1704067200,
                    close: '241.53',
                    change: '0',
                    percent_change: '0.25',
                    volume: '53,614,054',
                },
            ],
        },
        {
            symbol: 'AAPL',
            exchange: 'NASDAQ',
            range: 6,
            interval: 'daily',
            hl: 'en',
            gl: 'us',
        },
    );

    assert.equal(items.length, 1);
    assert.equal(items[0].position, 1);
    assert.equal(items[0].date, 1704067200);
    assert.equal(items[0].date_iso, '2024-01-01');
    assert.equal(items[0].close, 241.53);
    assert.equal(items[0].volume, 53614054);
    assert.equal(items[0].previous_close, 275.5);
    assert.deepEqual(items[0].result_counts, { prices: 1 });
});

test('falls back to request symbol and returns no rows for missing prices', () => {
    const items = buildHistoricalPriceDatasetItems(
        { currency: 'USD' },
        { symbol: 'VOO', range: 7 },
    );

    assert.deepEqual(items, []);
});
