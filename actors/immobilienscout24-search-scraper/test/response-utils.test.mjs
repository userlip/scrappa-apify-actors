import assert from 'node:assert/strict';
import test from 'node:test';

const responseUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const {
    buildImmobilienscout24DatasetItem,
    getImmobilienscout24Listings,
    getImmobilienscout24Page,
    getImmobilienscout24TotalPages,
    getImmobilienscout24TotalResults,
    limitImmobilienscout24SearchResponse,
} = await import(responseUtilsModule);

test('returns listings from top-level and wrapped Scrappa responses', () => {
    const results = [{ title: 'Berlin flat' }];

    assert.deepEqual(getImmobilienscout24Listings({ results }), results);
    assert.deepEqual(getImmobilienscout24Listings({ data: { results } }), results);
});

test('falls back to wrapped listings when top-level results are empty', () => {
    const results = [{ title: 'Nested Berlin flat' }];

    assert.deepEqual(getImmobilienscout24Listings({
        results: [],
        data: { results },
    }), results);
});

test('returns an empty listing array for unexpected response shape', () => {
    const originalDebug = console.debug;
    const messages = [];
    console.debug = (message) => messages.push(message);

    try {
        assert.deepEqual(getImmobilienscout24Listings({ success: false }), []);
        assert.deepEqual(getImmobilienscout24Listings(null), []);
    } finally {
        console.debug = originalDebug;
    }

    assert.deepEqual(messages, [
        'Unexpected ImmobilienScout24 response shape: expected "results" or "data.results" array.',
        'Unexpected ImmobilienScout24 response shape: expected "results" or "data.results" array.',
    ]);
});

test('reads pagination totals from top-level or wrapped response fields', () => {
    assert.equal(getImmobilienscout24TotalResults({ total_results: 5473 }), 5473);
    assert.equal(getImmobilienscout24TotalResults({ data: { total_results: 25 } }), 25);
    assert.equal(getImmobilienscout24Page({ page: 2 }), 2);
    assert.equal(getImmobilienscout24Page({ data: { page: '3' } }), 3);
    assert.equal(getImmobilienscout24TotalPages({ total_pages: 274 }), 274);
    assert.equal(getImmobilienscout24TotalPages({ data: { total_pages: 4 } }), 4);
});

test('adds table-friendly dataset aliases while preserving the raw listing', () => {
    const listing = {
        id: 'estate_123',
        online_id: '2paau5t',
        title: 'Moderne 3-Zimmerwohnung',
        price: 2170.82,
        price_formatted: '2.171 EUR (Kaltmiete)',
        rooms: 3.5,
        rooms_max: 3.5,
        size_m2: 113.81,
        size_m2_max: 113.81,
        address: 'Mitte, 10117, Berlin',
        lat: 52.511009,
        lon: 13.402116,
        image_url: 'https://example.com/image.jpg',
        url: 'https://www.immobilienscout24.de/expose/164121830',
        is_private: false,
        published: 'vor 12 Tagen',
    };
    const originalListing = structuredClone(listing);
    const datasetItem = buildImmobilienscout24DatasetItem(listing, {
        location: 'Berlin',
        type: 'apartment-rent',
        price_min: 500,
        price_max: 1500,
        rooms_min: 1.5,
        rooms_max: 4,
        size_min: 45,
        size_max: 120,
        page: 1,
        per_page: 20,
    });

    assert.deepEqual(listing, originalListing);
    assert.deepEqual(datasetItem, {
        ...originalListing,
        latitude: 52.511009,
        longitude: 13.402116,
        request_location: 'Berlin',
        request_type: 'apartment-rent',
        request_price_min: 500,
        request_price_max: 1500,
        request_rooms_min: 1.5,
        request_rooms_max: 4,
        request_size_min: 45,
        request_size_max: 120,
        request_page: 1,
        request_per_page: 20,
    });
});

test('limits OUTPUT response listings without mutating the original response', () => {
    const results = [
        { online_id: '1', title: 'First listing' },
        { online_id: '2', title: 'Second listing' },
    ];
    const response = {
        success: true,
        total_results: 5473,
        total_pages: 274,
        results,
    };
    const originalResponse = structuredClone(response);

    assert.deepEqual(limitImmobilienscout24SearchResponse(response, 1), {
        ...response,
        results: [results[0]],
    });
    assert.deepEqual(response, originalResponse);
});

test('limits wrapped OUTPUT response listings without mutating the original response', () => {
    const results = [
        { online_id: '1', title: 'First listing' },
        { online_id: '2', title: 'Second listing' },
    ];
    const response = {
        success: true,
        data: {
            total_results: 2,
            results,
        },
    };
    const originalResponse = structuredClone(response);

    assert.deepEqual(limitImmobilienscout24SearchResponse(response, 1), {
        ...response,
        data: {
            ...response.data,
            results: [results[0]],
        },
    });
    assert.deepEqual(response, originalResponse);
});

test('limits both top-level and wrapped OUTPUT listings when both are present', () => {
    const topLevelResults = [
        { online_id: '1', title: 'Top first listing' },
        { online_id: '2', title: 'Top second listing' },
    ];
    const wrappedResults = [
        { online_id: '3', title: 'Wrapped first listing' },
        { online_id: '4', title: 'Wrapped second listing' },
    ];
    const response = {
        success: true,
        results: topLevelResults,
        data: {
            total_results: 4,
            results: wrappedResults,
        },
    };
    const originalResponse = structuredClone(response);

    assert.deepEqual(limitImmobilienscout24SearchResponse(response, 1), {
        ...response,
        results: [topLevelResults[0]],
        data: {
            ...response.data,
            results: [wrappedResults[0]],
        },
    });
    assert.deepEqual(response, originalResponse);
});
