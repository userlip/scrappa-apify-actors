import assert from 'node:assert/strict';
import test from 'node:test';

const chargingModule = process.env.TEST_SOURCE === 'src'
    ? '../src/charging.ts'
    : '../dist/charging.js';
const {
    BOOKING_HOTEL_RESULT_CHARGE_EVENT,
    pushErrorHotelItem,
    pushSuccessfulHotelItem,
} = await import(chargingModule);

test('charges successful hotel items with the hotel-result event after dataset save', async () => {
    const calls = [];
    const actor = {
        getChargingManager() {
            return {
                getPricingInfo() {
                    return { isPayPerEvent: true };
                },
            };
        },
        async pushData(data, eventName) {
            calls.push({ data, eventName });
            return { eventChargeLimitReached: false, chargedCount: 1 };
        },
    };

    const result = await pushSuccessfulHotelItem(actor, { title: 'Ritz Paris' });

    assert.deepEqual(result, { eventChargeLimitReached: false, chargedCount: 1 });
    assert.deepEqual(calls, [{
        data: { title: 'Ritz Paris' },
        eventName: BOOKING_HOTEL_RESULT_CHARGE_EVENT,
    }]);
});

test('saves successful hotel items without event when actor is not pay-per-event', async () => {
    const calls = [];
    const actor = {
        getChargingManager() {
            return {
                getPricingInfo() {
                    return { isPayPerEvent: false };
                },
            };
        },
        async pushData(data, eventName) {
            calls.push({ data, eventName });
            return {};
        },
    };

    await pushSuccessfulHotelItem(actor, { title: 'Ritz Paris' });

    assert.deepEqual(calls, [{
        data: { title: 'Ritz Paris' },
        eventName: undefined,
    }]);
});

test('saves error rows without charging event', async () => {
    const calls = [];
    const actor = {
        async pushData(data, eventName) {
            calls.push({ data, eventName });
            return {};
        },
    };

    await pushErrorHotelItem(actor, { request_success: false, error_message: 'failed' });

    assert.deepEqual(calls, [{
        data: { request_success: false, error_message: 'failed' },
        eventName: undefined,
    }]);
});
