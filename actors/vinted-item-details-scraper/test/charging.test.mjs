import assert from 'node:assert/strict';
import test from 'node:test';

const chargingModule = process.env.TEST_SOURCE === 'src'
    ? '../src/charging.ts'
    : '../dist/charging.js';
const {
    getVintedItemDetailChargeLimitStatus,
    pushSuccessfulVintedItemDetail,
    pushVintedItemDetailError,
    VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT,
} = await import(chargingModule);

test('charges successful Vinted item details with the item-detail-result event', async () => {
    const calls = [];
    const actor = {
        getChargingManager() {
            return {
                getPricingInfo() {
                    return { isPayPerEvent: true };
                },
                calculateMaxEventChargeCountWithinLimit() {
                    return 10;
                },
            };
        },
        async pushData(data, eventName) {
            calls.push({ data, eventName });
            return { eventChargeLimitReached: false, chargedCount: 1 };
        },
    };

    const result = await pushSuccessfulVintedItemDetail(actor, { title: 'Nike Air Max 90' }, 0);

    assert.deepEqual(result, {
        saved: true,
        statusMessage: null,
        chargedCount: 1,
        eventChargeLimitReached: false,
    });
    assert.deepEqual(calls, [{
        data: { title: 'Nike Air Max 90' },
        eventName: VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT,
    }]);
});

test('saves successful Vinted item details without event when actor is not pay-per-event', async () => {
    const calls = [];
    const actor = {
        getChargingManager() {
            return {
                getPricingInfo() {
                    return { isPayPerEvent: false };
                },
                calculateMaxEventChargeCountWithinLimit() {
                    return 0;
                },
            };
        },
        async pushData(data, eventName) {
            calls.push({ data, eventName });
            return {};
        },
    };

    const result = await pushSuccessfulVintedItemDetail(actor, { title: 'Nike Air Max 90' }, 0);

    assert.deepEqual(result, {
        saved: true,
        statusMessage: null,
        chargedCount: 1,
        eventChargeLimitReached: false,
    });
    assert.deepEqual(calls, [{
        data: { title: 'Nike Air Max 90' },
        eventName: undefined,
    }]);
});

test('returns pre-fetch charge-limit status for pay-per-event runs without remaining charges', () => {
    const actor = {
        getChargingManager() {
            return {
                getPricingInfo() {
                    return { isPayPerEvent: true };
                },
                calculateMaxEventChargeCountWithinLimit(eventName) {
                    assert.equal(eventName, VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT);
                    return 0;
                },
            };
        },
    };

    assert.equal(
        getVintedItemDetailChargeLimitStatus(actor, 4, 5),
        'Charge limit reached before fetching Vinted item detail request 6; 4 item detail result(s) were saved.',
    );
});

test('saves error rows without charging event', async () => {
    const calls = [];
    const actor = {
        async pushData(data, eventName) {
            calls.push({ data, eventName });
            return {};
        },
    };

    await pushVintedItemDetailError(actor, { request_success: false, error_message: 'failed' });

    assert.deepEqual(calls, [{
        data: { request_success: false, error_message: 'failed' },
        eventName: undefined,
    }]);
});
