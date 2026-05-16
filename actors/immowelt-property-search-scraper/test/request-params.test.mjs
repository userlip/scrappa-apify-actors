import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildImmoweltPropertySearchParams,
    DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT,
    describeImmoweltPropertySearchRequest,
    normalizeImmoweltPropertySearchInput,
} from '../dist/request-params.js';

test('uses the default Berlin apartment search when input is missing or empty', () => {
    assert.deepEqual(normalizeImmoweltPropertySearchInput(), DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT);
    assert.deepEqual(normalizeImmoweltPropertySearchInput({}), DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT);
    assert.deepEqual(normalizeImmoweltPropertySearchInput({ hello: 'world' }), DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT);
});

test('trims known string fields and preserves numeric pagination input', () => {
    assert.deepEqual(normalizeImmoweltPropertySearchInput({
        location: ' Berlin ',
        property_type: ' apartment ',
        page: 2,
        limit: 50,
    }), {
        location: 'Berlin',
        property_type: 'apartment',
        page: 2,
        limit: 50,
    });
});

test('preserves blank required string inputs so validation fails instead of defaulting', () => {
    const normalized = normalizeImmoweltPropertySearchInput({
        location: '   ',
        property_type: ' apartment ',
    });

    assert.deepEqual(normalized, {
        ...DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT,
        location: '',
        property_type: 'apartment',
    });
    assert.throws(() => buildImmoweltPropertySearchParams(normalized), /location is required/);
});

test('builds Immowelt search params', () => {
    assert.deepEqual(buildImmoweltPropertySearchParams({
        location: 'Berlin',
        property_type: 'apartment',
        page: '2',
        limit: '25',
    }), {
        location: 'Berlin',
        property_type: 'apartment',
        page: 2,
        limit: 25,
    });
});

test('validates required strings and pagination bounds', () => {
    assert.throws(() => buildImmoweltPropertySearchParams({
        location: '',
        property_type: 'apartment',
        page: 1,
        limit: 20,
    }), /location is required/);
    assert.throws(() => buildImmoweltPropertySearchParams({
        location: 'Berlin',
        property_type: 'apartment',
        page: 0,
        limit: 20,
    }), /page must be between 1 and 10000/);
    assert.throws(() => buildImmoweltPropertySearchParams({
        location: 'Berlin',
        property_type: 'apartment',
        page: 1,
        limit: 101,
    }), /limit must be between 1 and 100/);
});

test('describes Immowelt requests for logs', () => {
    assert.equal(
        describeImmoweltPropertySearchRequest({
            location: 'Berlin',
            property_type: 'apartment',
            page: 2,
            limit: 25,
        }),
        'apartment properties in Berlin (page 2, limit 25)',
    );
});
