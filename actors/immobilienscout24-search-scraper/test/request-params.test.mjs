import assert from 'node:assert/strict';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildImmobilienscout24SearchParams,
    DEFAULT_IMMOBILIENSCOUT24_SEARCH_INPUT,
    describeImmobilienscout24SearchRequest,
    normalizeImmobilienscout24SearchInput,
} = await import(requestParamsModule);

test('uses the default Berlin apartment search when input is missing or empty', () => {
    assert.deepEqual(normalizeImmobilienscout24SearchInput(), DEFAULT_IMMOBILIENSCOUT24_SEARCH_INPUT);
    assert.deepEqual(normalizeImmobilienscout24SearchInput({}), DEFAULT_IMMOBILIENSCOUT24_SEARCH_INPUT);
    assert.deepEqual(normalizeImmobilienscout24SearchInput({ hello: 'world' }), DEFAULT_IMMOBILIENSCOUT24_SEARCH_INPUT);
});

test('trims known string fields and preserves numeric filter input', () => {
    assert.deepEqual(normalizeImmobilienscout24SearchInput({
        location: ' Berlin ',
        type: ' apartment-rent ',
        rooms_min: ' 1.5 ',
        price_max: 1500,
        page: 2,
        per_page: 50,
    }), {
        location: 'Berlin',
        type: 'apartment-rent',
        rooms_min: '1.5',
        price_max: 1500,
        page: 2,
        per_page: 50,
    });
});

test('maps legacy aliases onto Scrappa ImmobilienScout24 parameter names', () => {
    assert.deepEqual(normalizeImmobilienscout24SearchInput({
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

test('prefers canonical Scrappa ImmobilienScout24 parameter names over legacy aliases', () => {
    assert.deepEqual(normalizeImmobilienscout24SearchInput({
        location: 'Berlin',
        property_type: ' apartment-buy ',
        type: 'house-rent',
        limit: 25,
        per_page: 10,
    }), {
        location: 'Berlin',
        type: 'house-rent',
        page: 1,
        per_page: 10,
    });
});

test('preserves blank required string inputs so validation fails instead of defaulting', () => {
    const normalized = normalizeImmobilienscout24SearchInput({
        location: '   ',
        type: ' apartment-rent ',
    });

    assert.deepEqual(normalized, {
        ...DEFAULT_IMMOBILIENSCOUT24_SEARCH_INPUT,
        location: '',
        type: 'apartment-rent',
    });
    assert.throws(() => buildImmobilienscout24SearchParams(normalized), /location is required/);
});

test('preserves null known inputs so validation fails instead of defaulting', () => {
    const normalized = normalizeImmobilienscout24SearchInput({
        location: null,
        type: ' apartment-rent ',
    });

    assert.deepEqual(normalized, {
        ...DEFAULT_IMMOBILIENSCOUT24_SEARCH_INPUT,
        location: null,
        type: 'apartment-rent',
    });
    assert.throws(() => buildImmobilienscout24SearchParams(normalized), /location must be a string/);
});

test('builds ImmobilienScout24 search params', () => {
    assert.deepEqual(buildImmobilienscout24SearchParams({
        location: 'Berlin',
        type: 'apartment-rent',
        price_min: '500',
        price_max: '1500',
        rooms_min: '1.5',
        rooms_max: 4,
        size_min: '45',
        size_max: 120,
        page: '2',
        per_page: '25',
    }), {
        location: 'Berlin',
        type: 'apartment-rent',
        price_min: 500,
        price_max: 1500,
        rooms_min: 1.5,
        rooms_max: 4,
        size_min: 45,
        size_max: 120,
        page: 2,
        per_page: 25,
    });
});

test('validates required strings and pagination bounds', () => {
    assert.throws(() => buildImmobilienscout24SearchParams({
        location: '',
        type: 'apartment-rent',
        page: 1,
        per_page: 20,
    }), /location is required/);
    assert.throws(() => buildImmobilienscout24SearchParams({
        location: 'Berlin',
        type: 'apartment',
        page: 1,
        per_page: 20,
    }), /type must be one of: apartment-rent, apartment-buy, house-rent, house-buy/);
    assert.throws(() => buildImmobilienscout24SearchParams({
        location: 'Berlin',
        type: 'apartment-rent',
        page: 0,
        per_page: 20,
    }), /page must be between 1 and 10000/);
    assert.throws(() => buildImmobilienscout24SearchParams({
        location: 'Berlin',
        type: 'apartment-rent',
        page: 1,
        per_page: 51,
    }), /per_page must be between 1 and 50/);
});

test('validates optional filter bounds and range ordering', () => {
    assert.throws(() => buildImmobilienscout24SearchParams({
        location: 'Berlin',
        type: 'apartment-rent',
        price_min: -1,
        page: 1,
        per_page: 20,
    }), /price_min must be between 0 and 100000000/);
    assert.throws(() => buildImmobilienscout24SearchParams({
        location: 'Berlin',
        type: 'apartment-rent',
        rooms_min: 'two',
        page: 1,
        per_page: 20,
    }), /rooms_min must be a number/);
    assert.throws(() => buildImmobilienscout24SearchParams({
        location: 'Berlin',
        type: 'apartment-rent',
        size_min: 80,
        size_max: 60,
        page: 1,
        per_page: 20,
    }), /size_min must be less than or equal to size_max/);
});

test('describes ImmobilienScout24 requests for logs', () => {
    assert.equal(
        describeImmobilienscout24SearchRequest({
            location: 'Berlin',
            type: 'apartment-rent',
            price_min: 500,
            price_max: 1500,
            rooms_min: 1.5,
            page: 2,
            per_page: 25,
        }),
        'apartment-rent properties in Berlin (page 2, per_page 25, price 500-1500, rooms >= 1.5)',
    );
});
