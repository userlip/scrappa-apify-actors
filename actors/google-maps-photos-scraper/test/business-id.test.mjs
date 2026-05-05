import assert from 'node:assert/strict';
import test from 'node:test';

import { normalizeBusinessId } from '../dist/business-id.js';

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

test('passes non-url values through for API validation', () => {
    assert.deepEqual(normalizeBusinessId('not-a-real-business-id'), {
        businessId: 'not-a-real-business-id',
        source: 'business_id',
    });
});
