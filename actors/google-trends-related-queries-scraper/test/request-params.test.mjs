import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildGoogleTrendsAutocompleteParams,
    buildGoogleTrendsRelatedQueriesParams,
    describeGoogleTrendsRelatedQueriesRequest,
    shouldIncludeAutocomplete,
} from '../dist/request-params.js';

test('builds params for a complete Google Trends related queries request', () => {
    assert.deepEqual(
        buildGoogleTrendsRelatedQueriesParams({
            query: ' coffee ',
            geo: ' us ',
            time_range: '1Y',
            hl: 'EN',
            search_type: 'YouTube',
        }),
        {
            q: 'coffee',
            geo: 'US',
            time_range: '1y',
            hl: 'en',
            search_type: 'youtube',
        },
    );
});

test('keeps q as a backwards-compatible alias for query', () => {
    assert.deepEqual(
        buildGoogleTrendsRelatedQueriesParams({
            q: 'bitcoin',
            geo: 'worldwide',
        }),
        {
            q: 'bitcoin',
            geo: 'Worldwide',
        },
    );
});

test('falls back to q when query is empty', () => {
    assert.deepEqual(
        buildGoogleTrendsRelatedQueriesParams({
            query: '   ',
            q: 'coffee',
            geo: 'US',
        }),
        {
            q: 'coffee',
            geo: 'US',
        },
    );
});

test('requires a non-empty query', () => {
    assert.throws(
        () => buildGoogleTrendsRelatedQueriesParams({ query: '   ' }),
        /query is required/,
    );
});

test('rejects invalid filters', () => {
    assert.throws(
        () => buildGoogleTrendsRelatedQueriesParams({ query: 'tesla', time_range: '2y' }),
        /time_range must be one of/,
    );
    assert.throws(
        () => buildGoogleTrendsRelatedQueriesParams({ query: 'tesla', hl: 'eng' }),
        /hl must be 2 characters or fewer/,
    );
    assert.throws(
        () => buildGoogleTrendsRelatedQueriesParams({ query: 'tesla', search_type: 'podcasts' }),
        /search_type must be one of/,
    );
});

test('builds autocomplete params from supported related-query filters only', () => {
    assert.deepEqual(
        buildGoogleTrendsAutocompleteParams({
            q: 'coffee',
            geo: 'US',
            time_range: '1y',
            hl: 'en',
            search_type: 'web',
        }),
        {
            q: 'coffee',
            geo: 'US',
            hl: 'en',
        },
    );
});

test('parses include_autocomplete as an optional boolean', () => {
    assert.equal(shouldIncludeAutocomplete({}), false);
    assert.equal(shouldIncludeAutocomplete({ include_autocomplete: true }), true);
    assert.throws(
        () => shouldIncludeAutocomplete({ include_autocomplete: 'true' }),
        /include_autocomplete must be a boolean/,
    );
});

test('describes related query requests for logs', () => {
    assert.equal(
        describeGoogleTrendsRelatedQueriesRequest({
            q: 'tesla',
            geo: 'US',
            time_range: '1y',
            hl: 'en',
            search_type: 'web',
        }),
        '"tesla" (geo=US, time_range=1y, hl=en, search_type=web)',
    );
});
