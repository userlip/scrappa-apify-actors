import assert from 'node:assert/strict';
import test from 'node:test';

const chargingModule = process.env.TEST_SOURCE === 'src'
    ? '../src/charging.ts'
    : '../dist/charging.js';
const {
    REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT,
    getChargeLimitStatus,
    pushChargedProperty,
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
                    assert.equal(eventName, REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT);
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
        'Charge limit reached before fetching Redfin property 2; 4 property detail result(s) were saved.',
    );
});

test('pushes property without charge event when actor is not pay-per-event', async () => {
    const actor = createActorMock({ isPayPerEvent: false });
    const property = { id: 1 };

    const result = await pushChargedProperty(actor, property, 0);

    assert.deepEqual(result, { saved: true, statusMessage: null });
    assert.deepEqual(actor.pushed, [{ data: property, eventName: undefined }]);
});

test('reports exhausted pay-per-event charge limit while pushing property', async () => {
    const actor = createActorMock({
        isPayPerEvent: true,
        pushResults: [
            { chargedCount: 0, eventChargeLimitReached: true },
        ],
    });

    const result = await pushChargedProperty(actor, { id: 1 }, 0);

    assert.deepEqual(result, {
        saved: false,
        statusMessage: 'Charge limit reached before saving Redfin property detail result 1.',
    });
    assert.deepEqual(actor.pushed, [
        { data: { id: 1 }, eventName: REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT },
    ]);
});
