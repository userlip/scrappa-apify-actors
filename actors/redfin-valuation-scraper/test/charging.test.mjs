import assert from 'node:assert/strict';
import test from 'node:test';

const chargingModule = process.env.TEST_SOURCE === 'src'
    ? '../src/charging.ts'
    : '../dist/charging.js';
const {
    REDFIN_VALUATION_RESULT_CHARGE_EVENT,
    getChargeLimitStatus,
    pushChargedValuation,
} = await import(chargingModule);

function createActorMock({ isPayPerEvent, limitCount = 0, pushResult }) {
    const pushed = [];

    return {
        pushed,
        getChargingManager() {
            return {
                getPricingInfo() {
                    return { isPayPerEvent };
                },
                calculateMaxEventChargeCountWithinLimit(eventName) {
                    assert.equal(eventName, REDFIN_VALUATION_RESULT_CHARGE_EVENT);
                    return limitCount;
                },
            };
        },
        async pushData(data, eventName) {
            pushed.push({ data, eventName });
            return pushResult ?? {
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

test('charge limit status reports exhausted charge limit before fetching', () => {
    const actor = createActorMock({ isPayPerEvent: true, limitCount: 0 });

    assert.equal(
        getChargeLimitStatus(actor, 4, 1),
        'Charge limit reached before fetching Redfin valuation 2; 4 valuation result(s) were saved.',
    );
});

test('pushes valuation without charge event when actor is not pay-per-event', async () => {
    const actor = createActorMock({ isPayPerEvent: false });
    const item = { property_id: 194191988 };

    const result = await pushChargedValuation(actor, item);

    assert.deepEqual(result, { saved: true, statusMessage: null });
    assert.deepEqual(actor.pushed, [{ data: item, eventName: undefined }]);
});

test('pushes valuation with pay-per-event charge', async () => {
    const actor = createActorMock({ isPayPerEvent: true });
    const item = { property_id: 194191988 };

    const result = await pushChargedValuation(actor, item);

    assert.deepEqual(result, { saved: true, statusMessage: null });
    assert.deepEqual(actor.pushed, [{ data: item, eventName: REDFIN_VALUATION_RESULT_CHARGE_EVENT }]);
});

test('reports charge limit before saving valuation', async () => {
    const actor = createActorMock({
        isPayPerEvent: true,
        pushResult: { chargedCount: 0, eventChargeLimitReached: true },
    });

    const result = await pushChargedValuation(actor, { property_id: 194191988 });

    assert.deepEqual(result, {
        saved: false,
        statusMessage: 'Charge limit reached before saving the next Redfin valuation result.',
    });
});
