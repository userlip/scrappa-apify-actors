import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildGoogleTrendsAutocompleteParams,
    describeGoogleTrendsAutocompleteRequest,
} from '../dist/request-params.js';

test('builds params for a complete Google Trends autocomplete request', () => {
    assert.deepEqual(
        buildGoogleTrendsAutocompleteParams({
            query: ' tesla ',
            geo: ' us ',
            hl: 'EN',
        }),
        {
            q: 'tesla',
            geo: 'US',
            hl: 'en',
        },
    );
});

test('keeps q as a backwards-compatible alias for query', () => {
    assert.deepEqual(
        buildGoogleTrendsAutocompleteParams({
            q: 'bitcoin',
            geo: 'worldwide',
        }),
        {
            q: 'bitcoin',
            geo: 'Worldwide',
            hl: 'en',
        },
    );
});

test('falls back to q when query is empty', () => {
    assert.deepEqual(
        buildGoogleTrendsAutocompleteParams({
            query: '   ',
            q: 'coffee',
        }),
        {
            q: 'coffee',
            geo: 'US',
            hl: 'en',
        },
    );
});

test('defaults geo and hl for direct API-compatible input', () => {
    assert.deepEqual(
        buildGoogleTrendsAutocompleteParams({ query: 'tesla' }),
        {
            q: 'tesla',
            geo: 'US',
            hl: 'en',
        },
    );
});

test('requires a non-empty query', () => {
    assert.throws(
        () => buildGoogleTrendsAutocompleteParams({ query: '   ' }),
        /query is required/,
    );
});

test('rejects invalid filters', () => {
    assert.throws(
        () => buildGoogleTrendsAutocompleteParams({ query: 'tesla', hl: 'eng' }),
        /hl must be 2 characters or fewer/,
    );
    assert.throws(
        () => buildGoogleTrendsAutocompleteParams({ query: 'tesla', geo: 123 }),
        /geo must be a string/,
    );
});

test('describes autocomplete requests for logs', () => {
    assert.equal(
        describeGoogleTrendsAutocompleteRequest({
            q: 'tesla',
            geo: 'US',
            hl: 'en',
        }),
        '"tesla" (geo=US, hl=en)',
    );
});
