import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildImmoweltDatasetItem,
    getImmoweltPage,
    getImmoweltPropertyListings,
    getImmoweltTotalPages,
    getImmoweltTotalResults,
    limitImmoweltPropertySearchResponse,
} from '../dist/response-utils.js';

test('returns listings from top-level and wrapped Scrappa responses', () => {
    const results = [{ title: 'Berlin flat' }];

    assert.deepEqual(getImmoweltPropertyListings({ results }), results);
    assert.deepEqual(getImmoweltPropertyListings({ data: { results } }), results);
});

test('falls back to wrapped listings when top-level results are empty', () => {
    const results = [{ title: 'Nested Berlin flat' }];

    assert.deepEqual(getImmoweltPropertyListings({
        results: [],
        data: { results },
    }), results);
});

test('returns an empty listing array for unexpected response shape', () => {
    const originalDebug = console.debug;
    const messages = [];
    console.debug = (message) => messages.push(message);

    try {
        assert.deepEqual(getImmoweltPropertyListings({ success: false }), []);
        assert.deepEqual(getImmoweltPropertyListings(null), []);
    } finally {
        console.debug = originalDebug;
    }

    assert.deepEqual(messages, [
        'Unexpected Immowelt response shape: expected "results" or "data.results" array.',
        'Unexpected Immowelt response shape: expected "results" or "data.results" array.',
    ]);
});

test('reads pagination totals from top-level or wrapped response fields', () => {
    assert.equal(getImmoweltTotalResults({ total_results: 5473 }), 5473);
    assert.equal(getImmoweltTotalResults({ data: { total_results: 25 } }), 25);
    assert.equal(getImmoweltPage({ page: 2 }), 2);
    assert.equal(getImmoweltPage({ data: { page: '3' } }), 3);
    assert.equal(getImmoweltTotalPages({ total_pages: 274 }), 274);
    assert.equal(getImmoweltTotalPages({ data: { total_pages: 4 } }), 4);
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
        url: 'https://www.immowelt.de/expose/2paau5t',
        is_private: false,
        published: '2026-05-11T19:02:59.797Z',
    };
    const originalListing = structuredClone(listing);
    const datasetItem = buildImmoweltDatasetItem(listing, {
        location: 'Berlin',
        property_type: 'apartment',
        page: 1,
        limit: 20,
    });

    assert.deepEqual(listing, originalListing);
    assert.deepEqual(datasetItem, {
        ...originalListing,
        latitude: 52.511009,
        longitude: 13.402116,
        request_location: 'Berlin',
        request_property_type: 'apartment',
        request_page: 1,
        request_limit: 20,
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

    assert.deepEqual(limitImmoweltPropertySearchResponse(response, 1), {
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

    assert.deepEqual(limitImmoweltPropertySearchResponse(response, 1), {
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

    assert.deepEqual(limitImmoweltPropertySearchResponse(response, 1), {
        ...response,
        results: [topLevelResults[0]],
        data: {
            ...response.data,
            results: [wrappedResults[0]],
        },
    });
    assert.deepEqual(response, originalResponse);
});
