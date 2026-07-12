import assert from 'node:assert/strict';
import test from 'node:test';

const sourceRoot = process.env.TEST_SOURCE === 'src' ? '../src' : '../dist';
const { buildGoogleHotelsAutocompleteRequest, paramsForQuery } = await import(`${sourceRoot}/request-params.js`);

test('normalizes array and singular queries while preserving first spelling', () => {
    const request = buildGoogleHotelsAutocompleteRequest({
        queries: [' Berlin ', 'PARIS', 'berlin', ''],
        q: 'paris, London',
        gl: 'DE',
        hl: 'EN',
        currency: 'eur',
        type: 'ALL',
    });

    assert.deepEqual(request, {
        queries: ['Berlin', 'PARIS', 'London'],
        commonParams: { gl: 'de', hl: 'en', currency: 'EUR', type: 'all' },
    });
    assert.deepEqual(paramsForQuery('Berlin', request.commonParams), {
        q: 'Berlin', gl: 'de', hl: 'en', currency: 'EUR', type: 'all',
    });
});

test('accepts comma-separated queries', () => {
    assert.deepEqual(
        buildGoogleHotelsAutocompleteRequest({ queries: 'Berlin, Paris, berlin' }).queries,
        ['Berlin', 'Paris'],
    );
});

test('requires a query and validates localization fields', () => {
    assert.throws(() => buildGoogleHotelsAutocompleteRequest({}), /At least one query/);
    assert.throws(() => buildGoogleHotelsAutocompleteRequest({ q: 'Berlin', gl: 'germany' }), /2-letter code/);
    assert.throws(() => buildGoogleHotelsAutocompleteRequest({ q: 'Berlin', type: 'places' }), /location, hotel, all/);
});
