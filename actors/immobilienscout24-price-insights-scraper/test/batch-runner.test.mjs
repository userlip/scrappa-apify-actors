import assert from 'node:assert/strict';
import test from 'node:test';
import { PRICE_INSIGHT_RESULT_EVENT, runPriceInsightsBatch } from '../dist/batch-runner.js';
import { ScrappaApiError } from '../dist/shared/index.js';

const response = (location) => ({
    success: true,
    location,
    geocode: '123',
    currency: 'EUR',
    prices: {
        apartment_rent_per_m2: 10,
        apartment_buy_per_m2: 4000,
        house_rent_per_m2: 12,
        house_buy_per_m2: 4200,
    },
});

test('continues after a failed location and charges only successful snapshots', async () => {
    const calls = [];
    const client = {
        async get(_endpoint, params) {
            if (params.location === 'Nowhere') throw new ScrappaApiError(404, 'Location not found');
            return response(params.location);
        },
    };
    const writer = {
        isPayPerEvent: () => true,
        async pushData(item, eventName) {
            calls.push({ item, eventName });
            return { chargedCount: 1, eventChargeLimitReached: false };
        },
    };

    const result = await runPriceInsightsBatch([
        { location: 'Berlin', index: 0 },
        { location: 'Nowhere', index: 1 },
        { location: 'Munich', index: 2 },
    ], client, writer);

    assert.equal(result.succeeded, 2);
    assert.deepEqual(result.failures, [{ location: 'Nowhere', message: 'Location not found', status: 404 }]);
    assert.equal(calls.length, 2);
    assert.ok(calls.every((call) => call.eventName === PRICE_INSIGHT_RESULT_EVENT));
});

test('writes successful snapshots without an event on non-PPE builds', async () => {
    const calls = [];
    const result = await runPriceInsightsBatch(
        [{ location: 'Berlin', index: 0 }],
        { get: async () => response('Berlin') },
        {
            isPayPerEvent: () => false,
            async pushData(item, eventName) {
                calls.push({ item, eventName });
                return { chargedCount: 0, eventChargeLimitReached: false };
            },
        },
    );

    assert.equal(result.succeeded, 1);
    assert.equal(calls[0].eventName, undefined);
});
