import assert from 'node:assert/strict';
import test from 'node:test';

const modulePath = process.env.TEST_SOURCE === 'src'
    ? '../src/charged-save.ts'
    : '../dist/charged-save.js';
const { ROUTE_RESULT_CHARGE_EVENT, createChargedRouteWriter } = await import(modulePath);

function actor({ isPayPerEvent = true, capacity = 10, result = { chargedCount: 1, eventChargeLimitReached: false } } = {}) {
    const calls = [];
    return {
        calls,
        getChargingManager() {
            return {
                getPricingInfo() { return { isPayPerEvent }; },
                calculateMaxEventChargeCountWithinLimit(event) {
                    assert.equal(event, ROUTE_RESULT_CHARGE_EVENT);
                    return capacity;
                },
            };
        },
        async pushData(item, eventName) {
            calls.push({ item, eventName });
            return result;
        },
    };
}

test('charges only after Apify confirms a stored route row', async () => {
    const mock = actor();
    const writer = createChargedRouteWriter(mock, mock);
    assert.equal(writer.canSave(), true);
    assert.deepEqual(await writer.save({ alternative_index: 0 }), { saved: true, chargedCount: 1, chargeLimitReached: false });
    assert.deepEqual(mock.calls, [{ item: { alternative_index: 0 }, eventName: ROUTE_RESULT_CHARGE_EVENT }]);
});

test('does not report a saved row or charge when the event is refused', async () => {
    const mock = actor({ result: { chargedCount: 0, eventChargeLimitReached: true } });
    const writer = createChargedRouteWriter(mock, mock);
    assert.equal(writer.canSave(), true);
    assert.deepEqual(await writer.save({ alternative_index: 0 }), { saved: false, chargedCount: 0, chargeLimitReached: true });
});

test('prevents a request when no event capacity remains and supports non-PPE output', async () => {
    const limited = actor({ capacity: 0 });
    const limitedWriter = createChargedRouteWriter(limited, limited);
    assert.equal(limitedWriter.canSave(), false);

    const free = actor({ isPayPerEvent: false });
    const freeWriter = createChargedRouteWriter(free, free);
    assert.deepEqual(await freeWriter.save({ alternative_index: 0 }), { saved: true, chargedCount: 1, chargeLimitReached: false });
    assert.deepEqual(free.calls, [{ item: { alternative_index: 0 }, eventName: undefined }]);
});
