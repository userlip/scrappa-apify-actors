import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildVintedItemDetailsDatasetItem,
    buildVintedItemDetailsErrorItem,
    getVintedItemDetails,
} = await import(responseUtilsModule);

const request = {
    itemId: '1234567890',
    params: {
        item_id: '1234567890',
        country: 'DE',
    },
    index: 0,
};

test('extracts details from common Scrappa response shapes', () => {
    assert.deepEqual(getVintedItemDetails({ data: { title: 'Item A' } }), { title: 'Item A' });
    assert.deepEqual(getVintedItemDetails({ data: { item: { title: 'Item B' } } }), { title: 'Item B' });
    assert.deepEqual(getVintedItemDetails({ item: { title: 'Item C' } }), { title: 'Item C' });
});

test('rejects failed Scrappa envelopes before item extraction', () => {
    assert.throws(
        () => getVintedItemDetails({ success: false, data: { item: { id: 123 } }, message: 'Item not found' }),
        /Item not found/,
    );
    assert.throws(
        () => getVintedItemDetails({ success: false, data: { item: { id: 123 } } }),
        /Scrappa response reported failure/,
    );
});

test('rejects successful envelopes that do not include item details', () => {
    assert.throws(
        () => getVintedItemDetails({ success: true, data: {} }),
        /Scrappa response did not include Vinted item details/,
    );
    assert.throws(
        () => getVintedItemDetails({ success: true, data: { item: {} } }),
        /Scrappa response did not include Vinted item details/,
    );
});

test('accepts sparse item details using supported output fields', () => {
    assert.deepEqual(getVintedItemDetails({ data: { item: { path: '/items/123' } } }), { path: '/items/123' });
    assert.deepEqual(getVintedItemDetails({ data: { item: { brand_title: 'Nike' } } }), { brand_title: 'Nike' });
    assert.deepEqual(getVintedItemDetails({ data: { item: { availability: 'InStock' } } }), { availability: 'InStock' });
    assert.deepEqual(getVintedItemDetails({ data: { item: { user: { login: 'seller123' } } } }), { user: { login: 'seller123' } });
});

test('builds normalized Vinted item details dataset item', () => {
    const item = buildVintedItemDetailsDatasetItem(
        {
            id: '1234567890',
            title: 'Nike Air Max 90',
            description: 'Very good condition',
            price: { amount: '45.00', currency_code: 'EUR' },
            total_item_price: { amount: '50.49', currency_code: 'EUR' },
            shipping_price: { amount: '3.49', currency_code: 'EUR' },
            service_fee: { amount: '2.00', currency_code: 'EUR' },
            brand_title: 'Nike',
            category: { name: 'Shoes' },
            size_title: 'EU 42',
            status: 'Very good',
            availability: 'available',
            url: 'https://www.vinted.de/items/1234567890-nike-air-max-90',
            photo: { url: 'https://images1.vinted.net/example.jpg' },
            user: {
                id: 98765432,
                login: 'seller123',
                feedback_count: 50,
                feedback_reputation: 4.8,
            },
            favourite_count: 15,
            view_count: 234,
        },
        request,
    );

    assert.equal(item.id, '1234567890');
    assert.equal(item.title, 'Nike Air Max 90');
    assert.equal(item.price_amount, '45.00');
    assert.equal(item.price_currency, 'EUR');
    assert.equal(item.total_item_price, '50.49');
    assert.equal(item.service_fee, '2.00');
    assert.equal(item.brand_name, 'Nike');
    assert.equal(item.category_name, 'Shoes');
    assert.equal(item.size_name, 'EU 42');
    assert.equal(item.condition, 'Very good');
    assert.equal(item.availability, 'available');
    assert.equal(item.image_url, 'https://images1.vinted.net/example.jpg');
    assert.equal(item.seller_login, 'seller123');
    assert.equal(item.request_item_id, '1234567890');
    assert.equal(item.request_country, 'DE');
    assert.equal(item.request_index, 0);
    assert.equal(item.request_success, true);
});

test('falls back to requested item ID when response ID is missing', () => {
    const item = buildVintedItemDetailsDatasetItem({ title: 'Missing ID' }, request);

    assert.equal(item.id, '1234567890');
});

test('builds uncharged item-level error rows', () => {
    assert.deepEqual(
        buildVintedItemDetailsErrorItem(request, new Error('Scrappa API error (404): Not found')),
        {
            id: '1234567890',
            request_item_id: '1234567890',
            request_country: 'DE',
            request_index: 0,
            request_success: false,
            error_message: 'Scrappa API error (404): Not found',
        },
    );
});
