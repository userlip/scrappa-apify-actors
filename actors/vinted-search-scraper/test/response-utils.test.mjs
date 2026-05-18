import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildVintedDatasetItem,
    getVintedItems,
    getVintedPagination,
} = await import(responseUtilsModule);

test('extracts listings from primary and wrapped response shapes', () => {
    assert.deepEqual(getVintedItems({ items: [{ title: 'Item A' }] }), [{ title: 'Item A' }]);
    assert.deepEqual(getVintedItems({ data: { items: [{ title: 'Item B' }] } }), [{ title: 'Item B' }]);
    assert.deepEqual(getVintedItems({}), []);
});

test('extracts pagination from primary and wrapped response shapes', () => {
    assert.deepEqual(getVintedPagination({ pagination: { total_pages: 3 } }), { total_pages: 3 });
    assert.deepEqual(getVintedPagination({ data: { pagination: { total_entries: 42 } } }), { total_entries: 42 });
});

test('builds normalized Vinted dataset item', () => {
    const item = buildVintedDatasetItem(
        {
            id: '1234567890',
            title: 'Nike Air Max 90',
            price: { amount: '45.00', currency_code: 'EUR' },
            total_item_price: { amount: '50.49', currency_code: 'EUR' },
            shipping_price: { amount: '3.49', currency_code: 'EUR' },
            service_fee: { amount: '2.00', currency_code: 'EUR' },
            brand_title: 'Nike',
            category: { name: 'Shoes' },
            size_title: 'EU 42',
            status: 'Very good',
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
        {
            query: 'nike shoes',
            country: 'DE',
            page: 2,
            per_page: 50,
            order: 'newest_first',
            price_to: 80,
        },
        {
            data: {
                pagination: {
                    total_pages: 20,
                    total_entries: 980,
                },
            },
        },
    );

    assert.equal(item.id, '1234567890');
    assert.equal(item.price_amount, '45.00');
    assert.equal(item.price_currency, 'EUR');
    assert.equal(item.total_item_price, '50.49');
    assert.equal(item.service_fee, '2.00');
    assert.equal(item.brand_name, 'Nike');
    assert.equal(item.category_name, 'Shoes');
    assert.equal(item.size_name, 'EU 42');
    assert.equal(item.condition, 'Very good');
    assert.equal(item.image_url, 'https://images1.vinted.net/example.jpg');
    assert.equal(item.seller_login, 'seller123');
    assert.equal(item.request_query, 'nike shoes');
    assert.equal(item.request_country, 'DE');
    assert.equal(item.request_page, 2);
    assert.equal(item.total_pages, 20);
    assert.equal(item.total_entries, 980);
});
