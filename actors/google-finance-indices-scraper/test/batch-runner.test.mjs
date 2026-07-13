import assert from 'node:assert/strict';
import test from 'node:test';

import { runIndicesBatch } from '../dist/batch-runner.js';
import { saveIndex } from '../dist/charged-save.js';

test('does not fetch, save, or charge when PPE capacity is exhausted', async () => {
    let fetches = 0;
    let saves = 0;
    const summary = await runIndicesBatch(['.INX'], { hl: 'en', gl: 'us' }, {
        getCapacity: () => 0,
        fetch: async () => { fetches += 1; return { data: [] }; },
        save: async () => { saves += 1; return { savedCount: 1, chargeLimitReached: false }; },
    });

    assert.equal(fetches, 0);
    assert.equal(saves, 0);
    assert.equal(summary.attempted, 0);
    assert.equal(summary.saved, 0);
    assert.equal(summary.charged, 0);
    assert.equal(summary.charge_limit_reached, true);
    assert.deepEqual(summary.outcomes, [{
        symbol: '.INX', status: 'not_attempted', error: 'Charge limit reached',
    }]);
});

test('treats a refused PPE dataset save as failed and uncharged', async () => {
    const writes = [];
    const summary = await runIndicesBatch(['.INX'], { hl: 'en', gl: 'us' }, {
        getCapacity: () => Infinity,
        fetch: async () => ({ data: [{ symbol: '.INX', exchange: 'INDEXSP' }] }),
        save: async (item) => {
            writes.push(item);
            return { savedCount: 0, chargeLimitReached: true };
        },
    });

    assert.equal(writes.length, 1);
    assert.equal(summary.saved, 0);
    assert.equal(summary.failed, 1);
    assert.equal(summary.charged, 0);
    assert.equal(summary.charge_limit_reached, true);
    assert.equal(summary.outcomes[0].status, 'failed');
});

test('retains the final charged PPE result and stops before later rows after its limit flag', async () => {
    const writes = [];
    const summary = await runIndicesBatch(['.INX', '.DJI'], { hl: 'en', gl: 'us' }, {
        getCapacity: () => Infinity,
        fetch: async () => ({ data: [
            { symbol: '.INX', exchange: 'INDEXSP' },
            { symbol: '.DJI', exchange: 'INDEXDJX' },
        ] }),
        save: async (item) => {
            writes.push(item);
            return { savedCount: 1, chargeLimitReached: true };
        },
    });

    assert.equal(writes.length, 1);
    assert.equal(summary.saved, 1);
    assert.equal(summary.charged, 1);
    assert.equal(summary.failed, 0);
    assert.equal(summary.charge_limit_reached, true);
    assert.deepEqual(summary.outcomes, [
        { symbol: '.INX', status: 'saved' },
        { symbol: '.DJI', status: 'not_attempted', error: 'Charge limit reached' },
    ]);
});

test('saveIndex charges only PPE dataset writes that Apify accepts', async () => {
    const nonPpeEvents = [];
    const nonPpe = await saveIndex(
        { id: 'non-ppe' },
        { getPricingInfo: () => ({ isPayPerEvent: false }) },
        { pushData: async (_item, event) => { nonPpeEvents.push(event); } },
    );
    const ppeEvents = [];
    const acceptedPpe = await saveIndex(
        { id: 'accepted' },
        { getPricingInfo: () => ({ isPayPerEvent: true }) },
        { pushData: async (_item, event) => {
            ppeEvents.push(event);
            return { chargedCount: 1, eventChargeLimitReached: true };
        } },
    );
    const refusedPpe = await saveIndex(
        { id: 'refused' },
        { getPricingInfo: () => ({ isPayPerEvent: true }) },
        { pushData: async () => ({ chargedCount: 0, eventChargeLimitReached: true }) },
    );

    assert.deepEqual(nonPpe, { savedCount: 1, chargeLimitReached: false });
    assert.deepEqual(nonPpeEvents, [undefined]);
    assert.deepEqual(acceptedPpe, { savedCount: 1, chargeLimitReached: true });
    assert.deepEqual(ppeEvents, ['index-result']);
    assert.deepEqual(refusedPpe, { savedCount: 0, chargeLimitReached: true });
});

test('saves lowercase upstream symbol once for normalized requested input', async () => {
    const writes = [];
    const summary = await runIndicesBatch(['.INX'], { hl: 'en', gl: 'us' }, {
        getCapacity: () => Infinity,
        fetch: async () => ({ data: [{ symbol: '.inx', exchange: 'INDEXSP', name: 'S&P 500', price: '6200' }] }),
        save: async (item) => { writes.push(item); return { savedCount: 1, chargeLimitReached: false }; },
    });

    assert.equal(summary.saved, 1);
    assert.equal(summary.charged, 1);
    assert.equal(writes[0].symbol, '.INX');
});

test('fetches every requested symbol separately and aggregates only matching rows', async () => {
    const fetches = [];
    const writes = [];
    const responses = {
        '.INX': { data: [{ symbol: '.INX', exchange: 'INDEXSP' }] },
        '.DJI': { data: [{ symbol: '.DJI', exchange: 'INDEXDJX' }] },
        '.IXIC': { data: [
            { symbol: '.IXIC', exchange: 'INDEXNASDAQ' },
            { symbol: '.RUT', exchange: 'INDEXRUSSELL' },
        ] },
    };
    const summary = await runIndicesBatch(['.INX', '.DJI', '.IXIC'], { hl: 'en', gl: 'us' }, {
        getCapacity: () => Infinity,
        fetch: async (symbol) => {
            fetches.push(symbol);
            return responses[symbol];
        },
        save: async (item) => { writes.push(item); return { savedCount: 1, chargeLimitReached: false }; },
    });

    assert.deepEqual(fetches, ['.INX', '.DJI', '.IXIC']);
    assert.equal(summary.attempted, 3);
    assert.equal(summary.saved, 3);
    assert.equal(summary.charged, 3);
    assert.equal(summary.failed, 1);
    assert.deepEqual(writes.map((item) => item.symbol), ['.INX', '.DJI', '.IXIC']);
    assert.equal(writes.some((item) => item.symbol === '.RUT'), false);
});

test('processes every fetched response when capacity truncates a batch', async () => {
    const writes = [];
    const summary = await runIndicesBatch(['.INX', '.DJI', '.IXIC'], { hl: 'en', gl: 'us' }, {
        getCapacity: () => 2,
        fetch: async (symbol) => ({ data: [{ symbol, exchange: 'INDEX' }] }),
        save: async (item) => { writes.push(item); return { savedCount: 1, chargeLimitReached: false }; },
    });

    assert.equal(summary.attempted, 2);
    assert.equal(summary.saved, 2);
    assert.equal(summary.charged, 2);
    assert.equal(summary.charge_limit_reached, true);
    assert.deepEqual(writes.map((item) => item.symbol), ['.INX', '.DJI']);
    assert.deepEqual(summary.outcomes.find((outcome) => outcome.symbol === '.IXIC'), {
        symbol: '.IXIC', status: 'not_attempted', error: 'Charge limit reached',
    });
});

test('continues after one single-symbol request fails and never charges it', async () => {
    const fetches = [];
    const writes = [];
    const summary = await runIndicesBatch(['.INX', '.DJI', '.IXIC'], { hl: 'en', gl: 'us' }, {
        getCapacity: () => Infinity,
        fetch: async (symbol) => {
            fetches.push(symbol);
            if (symbol === '.DJI') throw new Error('Scrappa request failed after retries');
            return { data: [{ symbol, exchange: 'INDEX' }] };
        },
        save: async (item) => { writes.push(item); return { savedCount: 1, chargeLimitReached: false }; },
    });

    assert.deepEqual(fetches, ['.INX', '.DJI', '.IXIC']);
    assert.equal(summary.attempted, 3);
    assert.equal(summary.saved, 2);
    assert.equal(summary.charged, 2);
    assert.equal(summary.failed, 1);
    assert.deepEqual(writes.map((item) => item.symbol), ['.INX', '.IXIC']);
    assert.deepEqual(summary.outcomes.find((outcome) => outcome.symbol === '.DJI'), {
        symbol: '.DJI', status: 'failed', error: 'Scrappa request failed after retries',
    });
});

test('deduplicates rows and never charges mismatched symbols', async () => {
    let saves = 0;
    const summary = await runIndicesBatch(['.INX'], { hl: 'en', gl: 'us' }, {
        getCapacity: () => Infinity,
        fetch: async () => ({ data: [
            { symbol: '.INX', exchange: 'INDEXSP' },
            { symbol: '.INX', exchange: 'INDEXSP' },
            { symbol: '.DJI', exchange: 'INDEXDJX' },
        ] }),
        save: async () => { saves += 1; return { savedCount: 1, chargeLimitReached: false }; },
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
        fetch: async () => ({ indices: [
            { symbol: '.INX', exchange: 'INDEXSP' },
            { symbol: '.DJI', exchange: 'INDEXDJX' },
            { symbol: '.INX', exchange: 'INDEXSP' },
        ] }),
        save: async (item) => { writes.push(item); return { savedCount: 1, chargeLimitReached: false }; },
    });

    assert.equal(summary.saved, 2);
    assert.equal(summary.charged, 2);
    assert.equal(summary.duplicate, 1);
    assert.equal(summary.failed, 0);
    assert.deepEqual(writes.map((item) => item.requested_symbol), ['.INX', '.DJI']);
});

test('handles the indices response container for a requested symbol', async () => {
    const summary = await runIndicesBatch(['.INX'], { hl: 'en', gl: 'us' }, {
        getCapacity: () => Infinity,
        fetch: async () => ({ indices: [{ symbol: '.INX', exchange: 'INDEXSP' }] }),
        save: async () => ({ savedCount: 1, chargeLimitReached: false }),
    });

    assert.equal(summary.saved, 1);
    assert.equal(summary.charged, 1);
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
