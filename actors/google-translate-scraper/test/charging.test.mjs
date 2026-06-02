import assert from 'node:assert/strict';
import test from 'node:test';

const chargingModule = process.env.TEST_SOURCE === 'src'
    ? '../src/charging.ts'
    : '../dist/charging.js';
const {
    TRANSLATION_RESULT_CHARGE_EVENT,
    getTranslationChargeLimitStatus,
    pushTranslationResult,
} = await import(chargingModule);

function createActorMock({ isPayPerEvent, limitCount = 1, pushResult } = {}) {
    const pushed = [];

    return {
        pushed,
        getChargingManager() {
            return {
                getPricingInfo() {
                    return { isPayPerEvent };
                },
                calculateMaxEventChargeCountWithinLimit(eventName) {
                    assert.equal(eventName, TRANSLATION_RESULT_CHARGE_EVENT);
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

const successItem = {
    success: true,
    index: 0,
    text: 'Hello',
    translated_text: 'Hallo',
    source: 'en',
    target: 'de',
    error: null,
    status_code: null,
};

test('charge limit status is null when not pay-per-event', () => {
    const actor = createActorMock({ isPayPerEvent: false, limitCount: 0 });

    assert.equal(getTranslationChargeLimitStatus(actor, 2, 4), null);
});

test('charge limit status reports exhausted charge limit', () => {
    const actor = createActorMock({ isPayPerEvent: true, limitCount: 0 });

    assert.equal(
        getTranslationChargeLimitStatus(actor, 2, 4),
        'Charge limit reached before fetching the next translation; 2 of 4 translation item(s) were processed.',
    );
});

test('pushes successful translation with pay-per-event charge', async () => {
    const actor = createActorMock({ isPayPerEvent: true });

    const result = await pushTranslationResult(actor, successItem);

    assert.deepEqual(result, { saved: true, statusMessage: null });
    assert.deepEqual(actor.pushed, [{ data: successItem, eventName: TRANSLATION_RESULT_CHARGE_EVENT }]);
});

test('pushes failed translation without pay-per-event charge', async () => {
    const actor = createActorMock({ isPayPerEvent: true });
    const failureItem = { ...successItem, success: false, translated_text: null, error: 'Failed', status_code: 503 };

    const result = await pushTranslationResult(actor, failureItem);

    assert.deepEqual(result, { saved: true, statusMessage: null });
    assert.deepEqual(actor.pushed, [{ data: failureItem, eventName: undefined }]);
});

test('reports charge limit before saving successful translation', async () => {
    const actor = createActorMock({
        isPayPerEvent: true,
        pushResult: { chargedCount: 0, eventChargeLimitReached: true },
    });

    const result = await pushTranslationResult(actor, successItem);

    assert.deepEqual(result, {
        saved: false,
        statusMessage: 'Charge limit reached before saving translation 1; stopping batch without writing uncharged success results.',
    });
});
