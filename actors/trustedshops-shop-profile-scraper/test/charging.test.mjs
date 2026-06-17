import assert from 'node:assert/strict';
import test from 'node:test';

const chargingModule = process.env.TEST_SOURCE === 'src'
    ? '../src/charging.ts'
    : '../dist/charging.js';
const {
    SHOP_PROFILE_RESULT_CHARGE_EVENT,
    pushChargedItems,
} = await import(chargingModule);

function makeDataset({ isPayPerEvent, chargeResult } = {}) {
    const calls = [];
    return {
        calls,
        isPayPerEvent: () => isPayPerEvent ?? false,
        pushData: async (items, eventName) => {
            calls.push({ items, eventName });
            return chargeResult;
        },
    };
}

test('skips empty item batches', async () => {
    const dataset = makeDataset({ isPayPerEvent: true });
    const result = await pushChargedItems(dataset, []);

    assert.deepEqual(result, { savedCount: 0, statusMessage: null });
    assert.deepEqual(dataset.calls, []);
});

test('pushes free-mode items without charge event', async () => {
    const dataset = makeDataset({ isPayPerEvent: false });
    const result = await pushChargedItems(dataset, [{ id: 1 }]);

    assert.deepEqual(result, { savedCount: 1, statusMessage: null });
    assert.equal(dataset.calls[0].eventName, undefined);
});

test('pushes paid-mode items with shop profile charge event', async () => {
    const dataset = makeDataset({
        isPayPerEvent: true,
        chargeResult: { eventChargeLimitReached: false, chargedCount: 1 },
    });
    const result = await pushChargedItems(dataset, [{ id: 1 }]);

    assert.deepEqual(result, { savedCount: 1, statusMessage: null });
    assert.equal(dataset.calls[0].eventName, SHOP_PROFILE_RESULT_CHARGE_EVENT);
});

test('reports partial save when charge limit is reached', async () => {
    const dataset = makeDataset({
        isPayPerEvent: true,
        chargeResult: { eventChargeLimitReached: true, chargedCount: 1 },
    });
    const result = await pushChargedItems(dataset, [{ id: 1 }, { id: 2 }]);

    assert.equal(result.savedCount, 1);
    assert.match(result.statusMessage, /Charge limit reached after saving 1 of 2/);
    assert.equal(dataset.calls[0].eventName, SHOP_PROFILE_RESULT_CHARGE_EVENT);
});
