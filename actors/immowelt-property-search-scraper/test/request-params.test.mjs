import assert from 'node:assert/strict';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildImmoweltPropertySearchParams,
    DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT,
    describeImmoweltPropertySearchRequest,
    normalizeImmoweltPropertySearchInput,
} = await import(requestParamsModule);

test('uses the default Berlin apartment search when input is missing or empty', () => {
    assert.deepEqual(normalizeImmoweltPropertySearchInput(), DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT);
    assert.deepEqual(normalizeImmoweltPropertySearchInput({}), DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT);
    assert.deepEqual(normalizeImmoweltPropertySearchInput({ hello: 'world' }), DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT);
});

test('trims known string fields and preserves numeric pagination input', () => {
    assert.deepEqual(normalizeImmoweltPropertySearchInput({
        location: ' Berlin ',
        type: ' apartment-rent ',
        page: 2,
        per_page: 50,
    }), {
        location: 'Berlin',
        type: 'apartment-rent',
        page: 2,
        per_page: 50,
    });
});

test('maps legacy aliases onto Scrappa Immowelt parameter names', () => {
    assert.deepEqual(normalizeImmoweltPropertySearchInput({
        location: ' Berlin ',
        property_type: ' apartment-buy ',
        page: 2,
        limit: 25,
    }), {
        location: 'Berlin',
        type: 'apartment-buy',
        page: 2,
        per_page: 25,
    });
});

test('preserves blank required string inputs so validation fails instead of defaulting', () => {
    const normalized = normalizeImmoweltPropertySearchInput({
        location: '   ',
        type: ' apartment-rent ',
    });

    assert.deepEqual(normalized, {
        ...DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT,
        location: '',
        type: 'apartment-rent',
    });
    assert.throws(() => buildImmoweltPropertySearchParams(normalized), /location is required/);
});

test('preserves null known inputs so validation fails instead of defaulting', () => {
    const normalized = normalizeImmoweltPropertySearchInput({
        location: null,
        type: ' apartment-rent ',
    });

    assert.deepEqual(normalized, {
        ...DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT,
        location: null,
        type: 'apartment-rent',
    });
    assert.throws(() => buildImmoweltPropertySearchParams(normalized), /location must be a string/);
});

test('builds Immowelt search params', () => {
    assert.deepEqual(buildImmoweltPropertySearchParams({
        location: 'Berlin',
        type: 'apartment-rent',
        page: '2',
        per_page: '25',
    }), {
        location: 'Berlin',
        type: 'apartment-rent',
        page: 2,
        per_page: 25,
    });
});

test('validates required strings and pagination bounds', () => {
    assert.throws(() => buildImmoweltPropertySearchParams({
        location: '',
        type: 'apartment-rent',
        page: 1,
        per_page: 20,
    }), /location is required/);
    assert.throws(() => buildImmoweltPropertySearchParams({
        location: 'Berlin',
        type: 'apartment',
        page: 1,
        per_page: 20,
    }), /type must be one of: apartment-rent, apartment-buy, house-rent, house-buy/);
    assert.throws(() => buildImmoweltPropertySearchParams({
        location: 'Berlin',
        type: 'apartment-rent',
        page: 0,
        per_page: 20,
    }), /page must be between 1 and 10000/);
    assert.throws(() => buildImmoweltPropertySearchParams({
        location: 'Berlin',
        type: 'apartment-rent',
        page: 1,
        per_page: 51,
    }), /per_page must be between 1 and 50/);
});

test('describes Immowelt requests for logs', () => {
    assert.equal(
        describeImmoweltPropertySearchRequest({
            location: 'Berlin',
            type: 'apartment-rent',
            page: 2,
            per_page: 25,
        }),
        'apartment-rent properties in Berlin (page 2, per_page 25)',
    );
});
