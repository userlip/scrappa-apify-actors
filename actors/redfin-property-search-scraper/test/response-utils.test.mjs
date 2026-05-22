import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const { buildRedfinDatasetItem, getRedfinPropertyListings, getRedfinSearchCount } = await import(responseUtilsModule);

test('extracts Redfin listings from data.properties', () => {
    const properties = [{ property_id: 12345 }, { property_id: 67890 }];
    assert.deepEqual(getRedfinPropertyListings({ data: { properties } }), properties);
});

test('extracts Redfin listings from top-level properties fallback', () => {
    const properties = [{ property_id: 12345 }];
    assert.deepEqual(getRedfinPropertyListings({ properties }), properties);
});

test('returns empty list when no properties are present', () => {
    assert.deepEqual(getRedfinPropertyListings({ data: {} }), []);
});

test('reads Redfin search count from wrapped or top-level response', () => {
    assert.equal(getRedfinSearchCount({ data: { count: '12' } }), 12);
    assert.equal(getRedfinSearchCount({ count: 3 }), 3);
    assert.equal(getRedfinSearchCount({ count: 'Infinity' }), null);
});

test('builds normalized Redfin dataset item', () => {
    const item = buildRedfinDatasetItem(
        {
            property_id: '12345',
            listing_id: '67890',
            address: '123 Main St',
            city: 'Seattle',
            state: 'WA',
            zip: 98101,
            price: '850000',
            beds: '3',
            baths: '2.5',
            sqft: '1800',
            lot_size: '5000',
            year_built: '1925',
            property_type: '1',
            status: 'Active',
            latitude: '47.6097',
            longitude: '-122.3331',
            url: 'https://www.redfin.com/WA/Seattle/example/home/12345',
            mls_number: 123456,
        },
        {
            region_id: 16163,
            region_type: 6,
            market: 'seattle',
            min_price: 100000,
            max_price: 900000,
            num_beds: 2,
            num_baths: 1.5,
            property_types: '1,2',
            status: 9,
            sold_within_days: 30,
            num_homes: 50,
            page: 1,
        },
        1,
    );

    assert.equal(item.property_id, 12345);
    assert.equal(item.listing_id, 67890);
    assert.equal(item.address, '123 Main St');
    assert.equal(item.zip, '98101');
    assert.equal(item.price, 850000);
    assert.equal(item.beds, 3);
    assert.equal(item.baths, 2.5);
    assert.equal(item.sqft, 1800);
    assert.equal(item.lot_size, 5000);
    assert.equal(item.year_built, 1925);
    assert.equal(item.property_type, 1);
    assert.equal(item.property_type_label, 'House');
    assert.equal(item.status, 'Active');
    assert.equal(item.latitude, 47.6097);
    assert.equal(item.longitude, -122.3331);
    assert.equal(item.url, 'https://www.redfin.com/WA/Seattle/example/home/12345');
    assert.equal(item.mls_number, '123456');
    assert.equal(item.request_search_index, 1);
    assert.equal(item.request_region_id, 16163);
    assert.equal(item.request_region_type, 6);
    assert.equal(item.request_market, 'seattle');
    assert.equal(item.request_status, 9);
    assert.equal(item.request_status_label, 'All');
    assert.equal(item.request_num_homes, 50);
});

test('ignores non-finite numeric strings in normalized fields', () => {
    const item = buildRedfinDatasetItem(
        {
            property_id: 'Infinity',
            price: '-Infinity',
            latitude: 'NaN',
        },
        { region_id: 16163, region_type: 6, market: 'seattle' },
        0,
    );

    assert.equal(item.property_id, null);
    assert.equal(item.price, null);
    assert.equal(item.latitude, null);
});
