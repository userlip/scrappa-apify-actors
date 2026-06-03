import assert from 'node:assert/strict';
import test from 'node:test';

import { getBusinessIdRequests, normalizeBusinessId } from '../dist/business-id.js';

test('keeps google_id business identifiers unchanged', () => {
    assert.deepEqual(normalizeBusinessId(' 0x808fba02425dad8f:0x6c296c66619367e0 '), {
        businessId: '0x808fba02425dad8f:0x6c296c66619367e0',
        source: 'business_id',
    });
});

test('keeps Google Place IDs unchanged', () => {
    assert.deepEqual(normalizeBusinessId('ChIJj61dQgK6j4AR4GeTYWZsKWw'), {
        businessId: 'ChIJj61dQgK6j4AR4GeTYWZsKWw',
        source: 'place_id',
    });
});

test('extracts google_id values from Google Maps URLs', () => {
    assert.deepEqual(
        normalizeBusinessId('https://www.google.com/maps/place/Googleplex/data=!4m2!3m1!1s0x808fba02425dad8f:0x6c296c66619367e0'),
        {
            businessId: '0x808fba02425dad8f:0x6c296c66619367e0',
            source: 'url',
        },
    );
});

test('extracts encoded Place IDs from URLs', () => {
    assert.deepEqual(
        normalizeBusinessId('https://www.google.com/maps/search/?api=1&query=Googleplex&query_place_id=ChIJj61dQgK6j4AR4GeTYWZsKWw'),
        {
            businessId: 'ChIJj61dQgK6j4AR4GeTYWZsKWw',
            source: 'url',
        },
    );
});

test('rejects Google Maps URLs without an extractable supported identifier', () => {
    assert.throws(
        () => normalizeBusinessId('https://www.google.com/maps/place/Googleplex/@37.4220656,-122.0862784,17z'),
        /must contain an extractable/,
    );
});

test('rejects Maps short links because they do not contain identifiers locally', () => {
    assert.throws(
        () => normalizeBusinessId('https://maps.app.goo.gl/exampleShortCode'),
        /must contain an extractable/,
    );
});

test('recognizes alternate Google Maps hosts without identifiers', () => {
    assert.throws(
        () => normalizeBusinessId('https://maps.google.com/maps/place/Googleplex/@37.4220656,-122.0862784,17z'),
        /must contain an extractable/,
    );
    assert.throws(
        () => normalizeBusinessId('https://www.google.com.au/maps/place/Googleplex/@37.4220656,-122.0862784,17z'),
        /must contain an extractable/,
    );
});

test('recognizes query-style Google Maps URLs without identifiers', () => {
    assert.throws(
        () => normalizeBusinessId('https://www.google.com/maps?cid=123456789'),
        /must contain an extractable/,
    );
});

test('decodes repeatedly encoded URLs before extracting identifiers', () => {
    const mapsUrl = 'https://www.google.com/maps/search/?api=1&query_place_id=ChIJj61dQgK6j4AR4GeTYWZsKWw';
    const encodedTwice = encodeURIComponent(encodeURIComponent(mapsUrl));

    assert.deepEqual(normalizeBusinessId(encodedTwice), {
        businessId: 'ChIJj61dQgK6j4AR4GeTYWZsKWw',
        source: 'url',
    });
});

test('rejects empty strings in the normalizer', () => {
    assert.throws(
        () => normalizeBusinessId('   '),
        /Business ID is required/,
    );
});

test('passes non-url values through for API validation', () => {
    assert.deepEqual(normalizeBusinessId('not-a-real-business-id'), {
        businessId: 'not-a-real-business-id',
        source: 'business_id',
    });
});

test('getBusinessIdRequests supports backward-compatible business_id input', () => {
    assert.deepEqual(
        getBusinessIdRequests({ business_id: ' 0x808fba02425dad8f:0x6c296c66619367e0 ' }),
        [
            {
                input_business_id: '0x808fba02425dad8f:0x6c296c66619367e0',
                business_id: '0x808fba02425dad8f:0x6c296c66619367e0',
                source: 'business_id',
            },
        ],
    );
});

test('getBusinessIdRequests combines business_id and business_ids inputs and deduplicates normalized IDs', () => {
    assert.deepEqual(
        getBusinessIdRequests({
            business_id: '0x808fba02425dad8f:0x6c296c66619367e0',
            business_ids: [
                'https://www.google.com/maps/place/Googleplex/data=!4m2!3m1!1s0x808fba02425dad8f:0x6c296c66619367e0',
                'https://www.google.com/maps/search/?api=1&query_place_id=ChIJj61dQgK6j4AR4GeTYWZsKWw',
            ],
        }),
        [
            {
                input_business_id: '0x808fba02425dad8f:0x6c296c66619367e0',
                business_id: '0x808fba02425dad8f:0x6c296c66619367e0',
                source: 'business_id',
            },
            {
                input_business_id: 'https://www.google.com/maps/search/?api=1&query_place_id=ChIJj61dQgK6j4AR4GeTYWZsKWw',
                business_id: 'ChIJj61dQgK6j4AR4GeTYWZsKWw',
                source: 'url',
            },
        ],
    );
});

test('getBusinessIdRequests keeps invalid Google Maps URLs as per-item validation failures', () => {
    assert.deepEqual(
        getBusinessIdRequests({ business_ids: ['https://www.google.com/maps/place/Googleplex/@37.4220656,-122.0862784,17z'] }),
        [
            {
                input_business_id: 'https://www.google.com/maps/place/Googleplex/@37.4220656,-122.0862784,17z',
                validation_error: 'Google Maps URL must contain an extractable 0x...:0x... business ID or ChIJ... place ID. Use a business_id from a Google Maps Search or Business Details actor run.',
            },
        ],
    );
});

test('getBusinessIdRequests caps unique business inputs per run', () => {
    assert.equal(
        getBusinessIdRequests({
            business_ids: Array.from({ length: 10 }, (_, index) => `business-${index}`),
        }).length,
        10,
    );
    assert.equal(
        getBusinessIdRequests({
            business_id: 'business-0',
            business_ids: Array.from({ length: 10 }, (_, index) => `business-${index}`),
        }).length,
        10,
    );
    assert.throws(
        () => getBusinessIdRequests({
            business_ids: Array.from({ length: 11 }, (_, index) => `business-${index}`),
        }),
        /business_ids must contain 10 unique items or fewer/,
    );
});
