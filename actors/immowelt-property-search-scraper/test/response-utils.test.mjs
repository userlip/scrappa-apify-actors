import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildImmoweltDatasetItem,
    getImmoweltPage,
    getImmoweltPropertyListings,
    getImmoweltTotalPages,
    getImmoweltTotalResults,
} from '../dist/response-utils.js';

test('returns listings from top-level and wrapped Scrappa responses', () => {
    const results = [{ title: 'Berlin flat' }];

    assert.deepEqual(getImmoweltPropertyListings({ results }), results);
    assert.deepEqual(getImmoweltPropertyListings({ data: { results } }), results);
});

test('returns an empty listing array for unexpected response shape', () => {
    const originalDebug = console.debug;
    const messages = [];
    console.debug = (message) => messages.push(message);

    try {
        assert.deepEqual(getImmoweltPropertyListings({ success: false }), []);
    } finally {
        console.debug = originalDebug;
    }

    assert.deepEqual(messages, [
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

    assert.deepEqual(buildImmoweltDatasetItem(listing, {
        location: 'Berlin',
        property_type: 'apartment',
        page: 1,
        limit: 20,
    }), {
        ...listing,
        latitude: 52.511009,
        longitude: 13.402116,
        request_location: 'Berlin',
        request_property_type: 'apartment',
        request_page: 1,
        request_limit: 20,
    });
});
