import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildRedfinValuationDatasetItem,
    buildRedfinValuationFailureItem,
    getRedfinValuationData,
    hasMeaningfulValuationData,
} = await import(responseUtilsModule);

test('extracts wrapped Redfin valuation data', () => {
    const data = { predictedValue: 850000 };
    assert.deepEqual(getRedfinValuationData({ data }), data);
});

test('uses top-level response as fallback valuation data', () => {
    assert.deepEqual(getRedfinValuationData({ predictedValue: 850000 }), { predictedValue: 850000 });
});

test('detects meaningful Redfin valuation fields', () => {
    assert.equal(hasMeaningfulValuationData({ predictedValue: '850000' }), true);
    assert.equal(hasMeaningfulValuationData({ predicted_value: '850000' }), true);
    assert.equal(hasMeaningfulValuationData({ last_sold_price: '750000' }), true);
    assert.equal(hasMeaningfulValuationData({ comparables: [] }), false);
});

test('builds normalized Redfin valuation dataset item', () => {
    const item = buildRedfinValuationDatasetItem(
        {
            data: {
                predictedValue: '850000',
                predictedValueLow: 800000,
                predictedValueHigh: '900000',
                lastSoldPrice: '750000',
                lastSoldDate: '2020-05-15',
                numBeds: '3',
                numBaths: '2.5',
                sqFt: { value: '1800' },
                lotSize: { value: '4500' },
                yearBuilt: '1920',
                comparables: [
                    { address: '456 Oak Ave', price: 875000 },
                ],
            },
        },
        {
            index: 1,
            property_id: 194191988,
            listing_id: 207388793,
            url: 'https://www.redfin.com/home/194191988',
        },
    );

    assert.equal(item.success, true);
    assert.equal(item.property_id, 194191988);
    assert.equal(item.listing_id, 207388793);
    assert.equal(item.predicted_value, 850000);
    assert.equal(item.predicted_value_low, 800000);
    assert.equal(item.predicted_value_high, 900000);
    assert.equal(item.last_sold_price, 750000);
    assert.equal(item.last_sold_date, '2020-05-15');
    assert.equal(item.beds, 3);
    assert.equal(item.baths, 2.5);
    assert.equal(item.sqft, 1800);
    assert.equal(item.lot_size, 4500);
    assert.equal(item.year_built, 1920);
    assert.equal(item.comparables_count, 1);
    assert.deepEqual(item.comparables, [{ address: '456 Oak Ave', price: 875000 }]);
    assert.equal(item.request_index, 1);
    assert.equal(item.request_property_id, 194191988);
    assert.equal(item.request_listing_id, 207388793);
    assert.equal(item.request_url, 'https://www.redfin.com/home/194191988');
});

test('ignores non-finite numeric strings in normalized fields', () => {
    const item = buildRedfinValuationDatasetItem(
        { data: { predictedValue: 'Infinity', numBeds: 'NaN' } },
        { index: 0, property_id: 194191988, listing_id: null, url: null },
    );

    assert.equal(item.predicted_value, null);
    assert.equal(item.beds, null);
});

test('builds Redfin valuation failure dataset item', () => {
    const item = buildRedfinValuationFailureItem(
        {
            index: 2,
            property_id: 194191988,
            listing_id: null,
            url: 'https://www.redfin.com/home/194191988',
        },
        {
            status: 404,
            message: 'Valuation unavailable',
        },
    );

    assert.equal(item.success, false);
    assert.equal(item.status, 404);
    assert.equal(item.message, 'Valuation unavailable');
    assert.equal(item.property_id, 194191988);
    assert.equal(item.predicted_value, null);
    assert.equal(item.comparables_count, 0);
    assert.deepEqual(item.comparables, []);
    assert.equal(item.request_index, 2);
    assert.equal(item.request_property_id, 194191988);
    assert.equal(item.request_url, 'https://www.redfin.com/home/194191988');
});
