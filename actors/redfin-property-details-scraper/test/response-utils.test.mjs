import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildRedfinPropertyDetailsDatasetItem,
    buildRedfinPropertyErrorDatasetItem,
    getRedfinPropertyDetails,
} = await import(responseUtilsModule);

const request = {
    index: 1,
    input: 'https://www.redfin.com/TN/Memphis/1549-Ely-St-38106/home/60791456',
    source: 'url',
    params: { property_id: 60791456 },
};

test('extracts Redfin property details from data object', () => {
    const property = { property_id: 60791456, address: '1549 Ely St' };
    assert.deepEqual(getRedfinPropertyDetails({ data: property }), property);
});

test('extracts Redfin property details from data array fallback', () => {
    const property = { property_id: 60791456 };
    assert.deepEqual(getRedfinPropertyDetails({ data: [property] }), property);
});

test('extracts Redfin property details from property fallback', () => {
    const property = { property_id: 60791456 };
    assert.deepEqual(getRedfinPropertyDetails({ property }), property);
});

test('returns null when no property details are present', () => {
    assert.equal(getRedfinPropertyDetails({ data: null }), null);
    assert.equal(getRedfinPropertyDetails({ data: [] }), null);
});

test('builds normalized Redfin property details dataset item', () => {
    const item = buildRedfinPropertyDetailsDatasetItem(
        {
            property_id: '60791456',
            address: '1549 Ely St',
            city: 'Memphis',
            state: 'TN',
            zip: 38106,
            country: 'US',
            price: '125000',
            price_label: '$125K',
            beds: '3',
            baths: '2',
            sqft: '1400',
            lot_size: '6000',
            year_built: '1954',
            property_type: '1',
            status: '1',
            status_label: 'Active',
            latitude: '35.098',
            longitude: '-90.012',
            url: 'https://www.redfin.com/TN/Memphis/1549-Ely-St-38106/home/60791456',
            photos: [{ url: 'https://example.com/photo.jpg' }],
            description: 'Property description',
        },
        request,
    );

    assert.equal(item.property_id, 60791456);
    assert.equal(item.zip, '38106');
    assert.equal(item.price, 125000);
    assert.equal(item.beds, 3);
    assert.equal(item.baths, 2);
    assert.equal(item.sqft, 1400);
    assert.equal(item.lot_size, 6000);
    assert.equal(item.year_built, 1954);
    assert.equal(item.property_type, 1);
    assert.equal(item.status, 1);
    assert.equal(item.status_label, 'Active');
    assert.equal(item.latitude, 35.098);
    assert.equal(item.longitude, -90.012);
    assert.deepEqual(item.photos, [{ url: 'https://example.com/photo.jpg' }]);
    assert.equal(item.request_property_index, 1);
    assert.equal(item.request_property_id, 60791456);
    assert.equal(item.request_source, 'url');
});

test('uses requested property ID when response omits property_id', () => {
    const item = buildRedfinPropertyDetailsDatasetItem({ address: 'Unknown' }, request);
    assert.equal(item.property_id, 60791456);
});

test('builds non-charged error dataset item', () => {
    assert.deepEqual(
        buildRedfinPropertyErrorDatasetItem(request, 'Property not found', 404),
        {
            success: false,
            property_id: 60791456,
            request_property_index: 1,
            request_property_id: 60791456,
            request_input: 'https://www.redfin.com/TN/Memphis/1549-Ely-St-38106/home/60791456',
            request_source: 'url',
            error: 'Property not found',
            status_code: 404,
        },
    );
});
