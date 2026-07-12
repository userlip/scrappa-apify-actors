import assert from 'node:assert/strict';
import test from 'node:test';

import { extractIndexRows, mapIndexRow } from '../dist/response-utils.js';

test('maps direct response rows and all documented value aliases into dataset items', () => {
    const rows = extractIndexRows({
        results: [{
            symbol: '.inx',
            name: 'S&P 500',
            exchange: 'indexsp',
            price: '$6,200.50',
            change: '-12.25',
            price_change_percent: '-0.20%',
            previous_close: '6,212.75',
            price_movement_direction: 'DOWN',
        }],
    });

    assert.equal(rows.length, 1);

    const item = mapIndexRow(
        rows[0],
        '.INX',
        { hl: 'en', gl: 'us' },
        '2026-07-12T00:00:00.000Z',
    );

    assert.deepEqual(item, {
        id: 'INDEXSP:.INX',
        requested_symbol: '.INX',
        symbol: '.INX',
        name: 'S&P 500',
        exchange: 'indexsp',
        current_price: 6200.5,
        price_change: -12.25,
        percent_change: -0.2,
        previous_close: 6212.75,
        movement_direction: 'DOWN',
        request_hl: 'en',
        request_gl: 'us',
        retrieved_at: '2026-07-12T00:00:00.000Z',
    });
});

test('extracts nested result containers and maps current field aliases', () => {
    const rows = extractIndexRows({
        data: {
            results: [{
                symbol: '.dji',
                exchange: 'indexdjx',
                current_price: '44,000',
                price_change: '100',
                percent_change: '0.23',
                movement_direction: 'UP',
            }],
        },
    });

    assert.equal(rows.length, 1);

    const item = mapIndexRow(
        rows[0],
        '.DJI',
        { hl: 'de', gl: 'de' },
        '2026-07-12T01:00:00.000Z',
    );

    assert.deepEqual(item, {
        id: 'INDEXDJX:.DJI',
        requested_symbol: '.DJI',
        symbol: '.DJI',
        name: null,
        exchange: 'indexdjx',
        current_price: 44000,
        price_change: 100,
        percent_change: 0.23,
        previous_close: null,
        movement_direction: 'UP',
        request_hl: 'de',
        request_gl: 'de',
        retrieved_at: '2026-07-12T01:00:00.000Z',
    });
});
