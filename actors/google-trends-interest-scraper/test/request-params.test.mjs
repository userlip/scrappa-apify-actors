import assert from 'node:assert/strict';
import test from 'node:test';

import { buildGoogleTrendsInterestParams, describeGoogleTrendsInterestRequest } from '../dist/request-params.js';

test('builds params for a complete Google Trends interest request', () => {
    assert.deepEqual(
        buildGoogleTrendsInterestParams({
            q: ' tesla ',
            geo: ' us ',
            time_range: '1Y',
            hl: 'EN',
            search_type: 'YouTube',
        }),
        {
            q: 'tesla',
            geo: 'US',
            time_range: '1y',
            hl: 'en',
            search_type: 'youtube',
        },
    );
});

test('supports worldwide geo label', () => {
    assert.deepEqual(
        buildGoogleTrendsInterestParams({
            q: 'bitcoin',
            geo: 'worldwide',
        }),
        {
            q: 'bitcoin',
            geo: 'Worldwide',
        },
    );
});

test('requires a non-empty query', () => {
    assert.throws(
        () => buildGoogleTrendsInterestParams({ q: '   ' }),
        /q is required/,
    );
});

test('rejects invalid filters', () => {
    assert.throws(
        () => buildGoogleTrendsInterestParams({ q: 'tesla', time_range: '2y' }),
        /time_range must be one of/,
    );
    assert.throws(
        () => buildGoogleTrendsInterestParams({ q: 'tesla', hl: 'eng' }),
        /hl must be 2 characters or fewer/,
    );
    assert.throws(
        () => buildGoogleTrendsInterestParams({ q: 'tesla', search_type: 'podcasts' }),
        /search_type must be one of/,
    );
});

test('describes interest requests for logs', () => {
    assert.equal(
        describeGoogleTrendsInterestRequest({
            q: 'tesla',
            geo: 'US',
            time_range: '1y',
            hl: 'en',
            search_type: 'web',
        }),
        '"tesla" (geo=US, time_range=1y, hl=en, search_type=web)',
    );
});
