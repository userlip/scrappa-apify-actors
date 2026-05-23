import assert from 'node:assert/strict';
import test from 'node:test';

const {
    FINANCE_SEARCH_RESULT_CHARGE_EVENT,
    pushSearchItems,
} = await import('../dist/charging.js');

function makeActor({ isPayPerEvent, chargeResult } = {}) {
    const calls = [];
    return {
        calls,
        getChargingManager: () => ({
            getPricingInfo: () => ({ isPayPerEvent: isPayPerEvent ?? false }),
        }),
        pushData: async (items, eventName) => {
            calls.push({ items, eventName });
            return chargeResult ?? { eventChargeLimitReached: false, chargedCount: Array.isArray(items) ? items.length : 1 };
        },
    };
}

test('skips empty Google Finance search item batches', async () => {
    const actor = makeActor({ isPayPerEvent: true });
    const result = await pushSearchItems(actor, []);

    assert.deepEqual(result, { pushed: true, statusMessage: null });
    assert.deepEqual(actor.calls, []);
});

test('pushes Google Finance search items without charge event in free mode', async () => {
    const actor = makeActor({ isPayPerEvent: false });
    const items = [{ symbol: 'AAPL' }, { symbol: 'TSLA' }];
    const result = await pushSearchItems(actor, items);

    assert.deepEqual(result, { pushed: true, statusMessage: null });
    assert.equal(actor.calls.length, 1);
    assert.deepEqual(actor.calls[0], { items, eventName: undefined });
});

test('pushes Google Finance search items with the finance search charge event', async () => {
    const actor = makeActor({
        isPayPerEvent: true,
        chargeResult: { eventChargeLimitReached: false, chargedCount: 2 },
    });
    const items = [{ symbol: 'AAPL' }, { symbol: 'TSLA' }];
    const result = await pushSearchItems(actor, items);

    assert.deepEqual(result, { pushed: true, statusMessage: null });
    assert.equal(actor.calls.length, 1);
    assert.deepEqual(actor.calls[0], { items, eventName: FINANCE_SEARCH_RESULT_CHARGE_EVENT });
});

test('reports charge limit before all Google Finance search results are saved', async () => {
    const actor = makeActor({
        isPayPerEvent: true,
        chargeResult: { eventChargeLimitReached: true, chargedCount: 1 },
    });
    const result = await pushSearchItems(actor, [{ symbol: 'AAPL' }, { symbol: 'TSLA' }]);

    assert.equal(result.pushed, false);
    assert.equal(result.statusMessage, 'Charge limit reached before saving all Google Finance search results.');
    assert.equal(actor.calls[0].eventName, FINANCE_SEARCH_RESULT_CHARGE_EVENT);
});
