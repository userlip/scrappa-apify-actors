import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildKleinanzeigenDatasetItem,
    getKleinanzeigenListings,
    limitKleinanzeigenSearchResponse,
    selectKleinanzeigenListings,
} = await import(responseUtilsModule);

test('extracts listings from primary and wrapped response shapes', () => {
    assert.deepEqual(getKleinanzeigenListings({ data: [{ title: 'Listing A' }] }), [{ title: 'Listing A' }]);
    assert.deepEqual(getKleinanzeigenListings({ listings: [{ title: 'Listing B' }] }), [{ title: 'Listing B' }]);
    assert.deepEqual(getKleinanzeigenListings({ results: [{ title: 'Listing C' }] }), [{ title: 'Listing C' }]);
    assert.deepEqual(getKleinanzeigenListings({ items: [{ title: 'Listing D' }] }), [{ title: 'Listing D' }]);
    assert.deepEqual(getKleinanzeigenListings({ data: { listings: [{ title: 'Listing E' }] } }), [{ title: 'Listing E' }]);
    assert.deepEqual(getKleinanzeigenListings({ data: { results: [{ title: 'Listing F' }] } }), [{ title: 'Listing F' }]);
    assert.deepEqual(getKleinanzeigenListings({ data: { items: [{ title: 'Listing G' }] } }), [{ title: 'Listing G' }]);
    assert.deepEqual(getKleinanzeigenListings({}), []);
});

test('falls back to later non-empty listing arrays when earlier response shapes are empty', () => {
    assert.deepEqual(
        getKleinanzeigenListings({
            listings: [],
            data: {
                listings: [{ title: 'Nested Listing' }],
            },
        }),
        [{ title: 'Nested Listing' }],
    );
    assert.deepEqual(
        getKleinanzeigenListings({
            data: [],
            listings: [],
            results: [{ title: 'Result Listing' }],
        }),
        [{ title: 'Result Listing' }],
    );
    assert.deepEqual(
        getKleinanzeigenListings({
            data: {
                listings: [],
                results: [],
                items: [],
            },
            listings: [],
            results: [],
            items: [],
        }),
        [],
    );
});

test('prefers populated listing arrays in documented response priority order', () => {
    assert.deepEqual(
        getKleinanzeigenListings({
            data: [{ title: 'Direct Data Listing' }],
            listings: [{ title: 'Top Listing' }],
            results: [{ title: 'Top Result' }],
            items: [{ title: 'Top Item' }],
            data_extra: [{ title: 'Ignored Listing' }],
        }),
        [{ title: 'Direct Data Listing' }],
    );
    assert.deepEqual(
        getKleinanzeigenListings({
            listings: [{ title: 'Top Listing' }],
            results: [{ title: 'Top Result' }],
            items: [{ title: 'Top Item' }],
            data: {
                listings: [{ title: 'Nested Listing' }],
            },
        }),
        [{ title: 'Top Listing' }],
    );
    assert.deepEqual(
        getKleinanzeigenListings({
            data: {
                results: [{ title: 'Nested Result' }],
                items: [{ title: 'Nested Item' }],
            },
        }),
        [{ title: 'Nested Result' }],
    );
    assert.deepEqual(
        getKleinanzeigenListings({
            listings: [{ title: 'Top Listing' }],
            results: [{ title: 'Top Result' }],
            items: [{ title: 'Top Item' }],
            data: {
                listings: [{ title: 'Nested Listing' }],
                results: [{ title: 'Nested Result' }],
                items: [{ title: 'Nested Item' }],
            },
        }),
        [{ title: 'Top Listing' }],
    );
});

test('returns the selected listing response source', () => {
    assert.deepEqual(
        selectKleinanzeigenListings({
            listings: [],
            data: {
                listings: [{ title: 'Nested Listing' }],
            },
        }),
        {
            source: 'data.listings',
            listings: [{ title: 'Nested Listing' }],
        },
    );
});

test('builds normalized Kleinanzeigen dataset item', () => {
    const item = buildKleinanzeigenDatasetItem(
        {
            id: '2987654321',
            title: 'Apple iPhone 15 Pro 256GB',
            price: '850 EUR',
            price_numeric: 850,
            location: '10115 Mitte',
            image: 'https://img.kleinanzeigen.de/example.jpg',
            description: 'Sehr guter Zustand',
            has_shipping: true,
        },
        {
            query: 'iphone',
            location: 'Berlin',
            category: 'elektronik',
            page: 2,
            price_min: 500,
            price_max: 900,
        },
        {
            meta: {
                results_count: 26,
            },
        },
    );

    assert.equal(item.id, '2987654321');
    assert.equal(item.title, 'Apple iPhone 15 Pro 256GB');
    assert.equal(item.price, '850 EUR');
    assert.equal(item.price_numeric, 850);
    assert.equal(item.location, '10115 Mitte');
    assert.equal(item.image_url, 'https://img.kleinanzeigen.de/example.jpg');
    assert.equal(item.description, 'Sehr guter Zustand');
    assert.equal(item.has_shipping, true);
    assert.equal(item.request_query, 'iphone');
    assert.equal(item.request_location, 'Berlin');
    assert.equal(item.request_category, 'elektronik');
    assert.equal(item.request_page, 2);
    assert.equal(item.request_price_min, 500);
    assert.equal(item.request_price_max, 900);
    assert.equal(item.results_count, 26);
});

test('limits response arrays to saved listing count', () => {
    assert.deepEqual(
        limitKleinanzeigenSearchResponse({ data: [{ id: 1 }, { id: 2 }] }, 1),
        { data: [{ id: 1 }] },
    );
    assert.deepEqual(
        limitKleinanzeigenSearchResponse({ data: { listings: [{ id: 1 }, { id: 2 }] } }, 1),
        { data: { listings: [{ id: 1 }] } },
    );
    assert.deepEqual(
        limitKleinanzeigenSearchResponse({ results: [{ id: 1 }, { id: 2 }] }, 1),
        { results: [{ id: 1 }] },
    );
});

test('limits only the selected response array when multiple shapes are present', () => {
    assert.deepEqual(
        limitKleinanzeigenSearchResponse({
            data: {
                cursor: 'next-page',
                listings: [{ id: 'data-listing-1' }, { id: 'data-listing-2' }],
                results: [{ id: 'data-result-1' }, { id: 'data-result-2' }],
                items: [{ id: 'data-item-1' }, { id: 'data-item-2' }],
            },
            listings: [{ id: 'listing-1' }, { id: 'listing-2' }],
            results: [{ id: 'result-1' }, { id: 'result-2' }],
            items: [{ id: 'item-1' }, { id: 'item-2' }],
        }, 1, 'listings'),
        {
            data: {
                cursor: 'next-page',
            },
            listings: [{ id: 'listing-1' }],
        },
    );
});
