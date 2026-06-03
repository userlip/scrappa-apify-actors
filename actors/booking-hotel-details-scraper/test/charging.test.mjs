import assert from 'node:assert/strict';
import test from 'node:test';

const chargingModule = process.env.TEST_SOURCE === 'src'
    ? '../src/charging.ts'
    : '../dist/charging.js';
const {
    BOOKING_HOTEL_RESULT_CHARGE_EVENT,
    getHotelChargeLimitStatus,
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

    const result = await pushSuccessfulHotelItem(actor, { title: 'Ritz Paris' }, 0);

    assert.deepEqual(result, {
        saved: true,
        statusMessage: null,
        chargedCount: 1,
        eventChargeLimitReached: false,
    });
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

    const result = await pushSuccessfulHotelItem(actor, { title: 'Ritz Paris' }, 0);

    assert.deepEqual(result, {
        saved: true,
        statusMessage: null,
        chargedCount: 1,
        eventChargeLimitReached: false,
    });
    assert.deepEqual(calls, [{
        data: { title: 'Ritz Paris' },
        eventName: undefined,
    }]);
});

test('reports unsaved hotel item when event charge limit is reached before save', async () => {
    const actor = {
        getChargingManager() {
            return {
                getPricingInfo() {
                    return { isPayPerEvent: true };
                },
                calculateMaxEventChargeCountWithinLimit() {
                    return 0;
                },
            };
        },
        async pushData() {
            return { eventChargeLimitReached: true, chargedCount: 0 };
        },
    };

    const result = await pushSuccessfulHotelItem(actor, { title: 'Ritz Paris' }, 2);

    assert.deepEqual(result, {
        saved: false,
        statusMessage: 'Charge limit reached before saving Booking.com hotel detail result 3.',
        chargedCount: 0,
        eventChargeLimitReached: true,
    });
});

test('returns pre-fetch charge-limit status for pay-per-event runs without remaining charges', () => {
    const actor = {
        getChargingManager() {
            return {
                getPricingInfo() {
                    return { isPayPerEvent: true };
                },
                calculateMaxEventChargeCountWithinLimit(eventName) {
                    assert.equal(eventName, BOOKING_HOTEL_RESULT_CHARGE_EVENT);
                    return 0;
                },
            };
        },
    };

    assert.equal(
        getHotelChargeLimitStatus(actor, 4, 5),
        'Charge limit reached before fetching Booking.com hotel detail request 6; 4 hotel detail result(s) were saved.',
    );
});

test('does not report pre-fetch charge-limit status outside pay-per-event runs', () => {
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
    };

    assert.equal(getHotelChargeLimitStatus(actor, 0, 0), null);
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
