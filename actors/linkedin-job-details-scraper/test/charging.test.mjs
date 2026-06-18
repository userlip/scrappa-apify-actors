import assert from 'node:assert/strict';
import test from 'node:test';

const chargingModule = process.env.ACTOR_TEST_TARGET === 'src'
    ? '../src/charging.ts'
    : '../dist/charging.js';
const {
    JOB_RESULT_CHARGE_EVENT,
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

test('skips pushing empty item batches', async () => {
    const dataset = makeDataset({ isPayPerEvent: true });
    const result = await pushChargedItems(dataset, []);

    assert.deepEqual(result, { savedCount: 0, statusMessage: null });
    assert.deepEqual(dataset.calls, []);
});

test('pushes free-mode items without charge event', async () => {
    const dataset = makeDataset({ isPayPerEvent: false });
    const items = [{ id: 1 }, { id: 2 }];
    const result = await pushChargedItems(dataset, items);

    assert.deepEqual(result, { savedCount: 2, statusMessage: null });
    assert.equal(dataset.calls.length, 1);
    assert.equal(dataset.calls[0].eventName, undefined);
    assert.deepEqual(dataset.calls[0].items, items);
});

test('pushes paid-mode items with job result charge event', async () => {
    const dataset = makeDataset({
        isPayPerEvent: true,
        chargeResult: { eventChargeLimitReached: false, chargedCount: 2 },
    });
    const items = [{ id: 1 }, { id: 2 }];
    const result = await pushChargedItems(dataset, items);

    assert.deepEqual(result, { savedCount: 2, statusMessage: null });
    assert.equal(dataset.calls.length, 1);
    assert.equal(dataset.calls[0].eventName, JOB_RESULT_CHARGE_EVENT);
});

test('handles paid-mode pushData implementations that return void', async () => {
    const dataset = makeDataset({ isPayPerEvent: true });
    const items = [{ id: 1 }];
    const result = await pushChargedItems(dataset, items);

    assert.deepEqual(result, { savedCount: 1, statusMessage: null });
    assert.equal(dataset.calls.length, 1);
    assert.equal(dataset.calls[0].eventName, JOB_RESULT_CHARGE_EVENT);
});

test('pushes uncharged paid-mode items without charge event', async () => {
    const dataset = makeDataset({ isPayPerEvent: true });
    const items = [{ success: false }];
    const result = await pushChargedItems(dataset, items, { chargeEvent: false });

    assert.deepEqual(result, { savedCount: 1, statusMessage: null });
    assert.equal(dataset.calls.length, 1);
    assert.equal(dataset.calls[0].eventName, undefined);
    assert.deepEqual(dataset.calls[0].items, items);
});

test('reports partial save when charge limit is reached', async () => {
    const dataset = makeDataset({
        isPayPerEvent: true,
        chargeResult: { eventChargeLimitReached: true, chargedCount: 1 },
    });
    const result = await pushChargedItems(dataset, [{ id: 1 }, { id: 2 }]);

    assert.equal(result.savedCount, 1);
    assert.match(result.statusMessage, /Charge limit reached after saving 1 of 2/);
    assert.equal(dataset.calls[0].eventName, JOB_RESULT_CHARGE_EVENT);
});
