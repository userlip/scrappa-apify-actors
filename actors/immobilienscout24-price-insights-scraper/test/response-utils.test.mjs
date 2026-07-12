import assert from 'node:assert/strict';
import test from 'node:test';
const modulePath = process.env.TEST_SOURCE === 'src' ? '../src/response-utils.ts' : '../dist/response-utils.js';
const { buildPriceInsightItem } = await import(modulePath);

const completeResponse = {
    success: true,
    location: 'Berlin',
    geocode: '1276003001',
    currency: 'EUR',
    prices: {
        apartment_rent_per_m2: 12.72,
        apartment_buy_per_m2: 4189.04,
        house_rent_per_m2: 16.51,
        house_buy_per_m2: 4394.87,
    },
};

test('maps the complete Scrappa response to a dataset item', () => {
    assert.deepEqual(buildPriceInsightItem(completeResponse, { location: 'berlin', index: 2 }), {
        location: 'Berlin',
        geocode: '1276003001',
        currency: 'EUR',
        apartment_rent_per_m2: 12.72,
        apartment_buy_per_m2: 4189.04,
        house_rent_per_m2: 16.51,
        house_buy_per_m2: 4394.87,
        request_location: 'berlin',
        request_index: 2,
    });
});

test('does not map incomplete responses', () => {
    assert.equal(buildPriceInsightItem({ ...completeResponse, prices: null }, { location: 'Berlin', index: 0 }), null);
});

test('does not map non-positive price benchmarks', () => {
    const prices = { ...completeResponse.prices, apartment_rent_per_m2: 0 };

    assert.equal(buildPriceInsightItem({ ...completeResponse, prices }, { location: 'Berlin', index: 0 }), null);
});
