import assert from 'node:assert/strict';
import test from 'node:test';

const chargingModule = process.env.TEST_SOURCE === 'src'
    ? '../src/charging.ts'
    : '../dist/charging.js';
const {
    REDFIN_PROPERTY_RESULT_CHARGE_EVENT,
    getChargeLimitStatus,
    pushChargedProperties,
} = await import(chargingModule);

function createActorMock({ isPayPerEvent, limitCount = 0, pushResults = [] }) {
    const pushed = [];

    return {
        pushed,
        getChargingManager() {
            return {
                getPricingInfo() {
                    return { isPayPerEvent };
                },
                calculateMaxEventChargeCountWithinLimit(eventName) {
                    assert.equal(eventName, REDFIN_PROPERTY_RESULT_CHARGE_EVENT);
                    return limitCount;
                },
            };
        },
        async pushData(data, eventName) {
            pushed.push({ data, eventName });
            return pushResults.shift() ?? {
                chargedCount: eventName ? 1 : 0,
                eventChargeLimitReached: false,
            };
        },
    };
}

test('charge limit status is null when actor is not pay-per-event', () => {
    const actor = createActorMock({ isPayPerEvent: false });

    assert.equal(getChargeLimitStatus(actor, 4, 1), null);
});

test('charge limit status is null when a pay-per-event charge remains', () => {
    const actor = createActorMock({ isPayPerEvent: true, limitCount: 1 });

    assert.equal(getChargeLimitStatus(actor, 4, 1), null);
});

test('charge limit status reports exhausted charge limit before fetching', () => {
    const actor = createActorMock({ isPayPerEvent: true, limitCount: 0 });

    assert.equal(
        getChargeLimitStatus(actor, 4, 1),
        'Charge limit reached before fetching Redfin search 2; 4 property result(s) were saved.',
    );
});

test('pushes all properties without charge event when actor is not pay-per-event', async () => {
    const actor = createActorMock({ isPayPerEvent: false });
    const properties = [{ id: 1 }, { id: 2 }];

    const result = await pushChargedProperties(actor, properties, 0);

    assert.deepEqual(result, { savedCount: 2, statusMessage: null });
    assert.deepEqual(actor.pushed, [{ data: properties, eventName: undefined }]);
});

test('stops pushing properties when pay-per-event charge limit is reached', async () => {
    const actor = createActorMock({
        isPayPerEvent: true,
        pushResults: [
            { chargedCount: 1, eventChargeLimitReached: false },
            { chargedCount: 0, eventChargeLimitReached: true },
        ],
    });

    const result = await pushChargedProperties(actor, [{ id: 1 }, { id: 2 }, { id: 3 }], 0);

    assert.deepEqual(result, {
        savedCount: 1,
        statusMessage: 'Charge limit reached before saving the next Redfin property result for search 1.',
    });
    assert.deepEqual(actor.pushed, [
        { data: { id: 1 }, eventName: REDFIN_PROPERTY_RESULT_CHARGE_EVENT },
        { data: { id: 2 }, eventName: REDFIN_PROPERTY_RESULT_CHARGE_EVENT },
    ]);
});
