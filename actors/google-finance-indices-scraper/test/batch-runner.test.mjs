import assert from 'node:assert/strict';
import test from 'node:test';

import { runIndicesBatch } from '../dist/batch-runner.js';

test('saves lowercase upstream symbol once for normalized requested input', async () => {
    const writes = [];
    const summary = await runIndicesBatch(['.INX'], { hl: 'en', gl: 'us' }, {
        getCapacity: () => Infinity,
        fetch: async () => ({
            data: [{
                symbol: '.inx',
                exchange: 'INDEXSP',
                name: 'S&P 500',
                price: '6200',
            }],
        }),
        save: async (item) => {
            writes.push(item);
            return { savedCount: 1, chargeLimitReached: false };
        },
    });

    assert.equal(summary.saved, 1);
    assert.equal(summary.charged, 1);
    assert.equal(writes[0].symbol, '.INX');
});

test('matches an exchange-qualified requested symbol against Scrappa response fields', async () => {
    const writes = [];
    const summary = await runIndicesBatch(['INDEXSP:.INX'], { hl: 'en', gl: 'us' }, {
        getCapacity: () => Infinity,
        fetch: async () => ({
            data: [{ symbol: '.INX', exchange: 'INDEXSP', name: 'S&P 500' }],
        }),
        save: async (item) => {
            writes.push(item);
            return { savedCount: 1, chargeLimitReached: false };
        },
    });

    assert.equal(summary.saved, 1);
    assert.equal(summary.failed, 0);
    assert.equal(writes[0].requested_symbol, 'INDEXSP:.INX');
});

test('deduplicates rows and never charges mismatched symbols', async () => {
    let saves = 0;
    const summary = await runIndicesBatch(['.INX'], { hl: 'en', gl: 'us' }, {
        getCapacity: () => Infinity,
        fetch: async () => ({
            data: [
                { symbol: '.INX', exchange: 'INDEXSP' },
                { symbol: '.INX', exchange: 'INDEXSP' },
                { symbol: '.DJI', exchange: 'INDEXDJX' },
            ],
        }),
        save: async () => {
            saves += 1;
            return { savedCount: 1, chargeLimitReached: false };
        },
    });

    assert.equal(saves, 1);
    assert.equal(summary.saved, 1);
    assert.equal(summary.duplicate, 1);
    assert.equal(summary.failed, 1);
});

test('saves and charges each unique default result when no indices are requested', async () => {
    const writes = [];
    const summary = await runIndicesBatch([], { hl: 'en', gl: 'us' }, {
        getCapacity: () => Infinity,
        fetch: async () => ({
            indices: [
                { symbol: '.INX', exchange: 'INDEXSP' },
                { symbol: '.DJI', exchange: 'INDEXDJX' },
                { symbol: '.INX', exchange: 'INDEXSP' },
            ],
        }),
        save: async (item) => {
            writes.push(item);
            return { savedCount: 1, chargeLimitReached: false };
        },
    });

    assert.equal(summary.saved, 2);
    assert.equal(summary.charged, 2);
    assert.equal(summary.duplicate, 1);
    assert.equal(summary.failed, 0);
    assert.deepEqual(writes.map((item) => item.requested_symbol), ['.INX', '.DJI']);
});

test('propagates an exhausted upstream request failure to the Actor failure handler', async () => {
    const upstreamError = new Error('Scrappa request failed after retries');

    await assert.rejects(
        runIndicesBatch(['.INX'], { hl: 'en', gl: 'us' }, {
            getCapacity: () => Infinity,
            fetch: async () => { throw upstreamError; },
            save: async () => ({ savedCount: 1, chargeLimitReached: false }),
        }),
        upstreamError,
    );
});
