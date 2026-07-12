import assert from 'node:assert/strict';
import test from 'node:test';

const sourceRoot = process.env.TEST_SOURCE === 'src' ? '../src' : '../dist';
const { buildGoogleHotelsAutocompleteRequest, paramsForQuery } = await import(`${sourceRoot}/request-params.js`);

test('normalizes array and singular queries while preserving first spelling', () => {
    const queries = [' Berlin ', 'PARIS', 'berlin', ''];
    const request = buildGoogleHotelsAutocompleteRequest({
        queries,
        q: ' London ',
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
    assert.deepEqual(queries, [' Berlin ', 'PARIS', 'berlin', '']);
});

test('preserves commas in the singular q compatibility input', () => {
    const request = buildGoogleHotelsAutocompleteRequest({ q: 'Paris, France' });

    assert.deepEqual(request.queries, ['Paris, France']);
    assert.equal(request.commonParams.type, 'all');
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

test('rejects more than 100 unique queries', () => {
    const queries = Array.from({ length: 101 }, (_, index) => `query ${index}`);
    assert.throws(
        () => buildGoogleHotelsAutocompleteRequest({ queries }),
        /maximum of 100 unique queries/,
    );
});
