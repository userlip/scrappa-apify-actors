import assert from 'node:assert/strict';
import test from 'node:test';

import { buildGoogleNewsParams, describeGoogleNewsRequest } from '../dist/request-params.js';

test('builds params for query searches', () => {
    assert.deepEqual(
        buildGoogleNewsParams({
            q: ' artificial intelligence ',
            gl: 'US',
            hl: 'EN',
            page: 2,
            so: 1,
        }),
        {
            q: 'artificial intelligence',
            gl: 'us',
            hl: 'en',
            page: 2,
            so: 1,
        },
    );
});

test('builds params for token searches', () => {
    assert.deepEqual(
        buildGoogleNewsParams({
            publication_token: ' CAAqExample ',
            gl: 'gb',
            hl: 'en',
            start: 10,
        }),
        {
            publication_token: 'CAAqExample',
            gl: 'gb',
            hl: 'en',
            start: 10,
        },
    );
});

test('requires q or a token parameter', () => {
    assert.throws(
        () => buildGoogleNewsParams({ gl: 'us', hl: 'en' }),
        /Provide q or one Google News token parameter/,
    );
});

test('rejects q combined with token parameters', () => {
    assert.throws(
        () => buildGoogleNewsParams({ q: 'markets', topic_token: 'CAAqExample' }),
        /Cannot use q with/,
    );
});

test('requires kgmid to be used alone and to have a valid prefix', () => {
    assert.throws(
        () => buildGoogleNewsParams({ kgmid: '/m/example', topic_token: 'CAAqExample' }),
        /kgmid must be used alone/,
    );
    assert.throws(
        () => buildGoogleNewsParams({ kgmid: 'example' }),
        /kgmid must start with \/m\/ or \/g\//,
    );
    assert.deepEqual(buildGoogleNewsParams({ kgmid: '/g/11example' }), { kgmid: '/g/11example' });
});

test('rejects invalid pagination and sort controls', () => {
    assert.throws(
        () => buildGoogleNewsParams({ q: 'markets', page: 1, start: 0 }),
        /Cannot use both page and start/,
    );
    assert.throws(
        () => buildGoogleNewsParams({ q: 'markets', page: 0 }),
        /page must be greater than or equal to 1/,
    );
    assert.throws(
        () => buildGoogleNewsParams({ q: 'markets', so: 2 }),
        /so must be between 0 and 1/,
    );
});

test('rejects invalid country and language codes', () => {
    assert.throws(
        () => buildGoogleNewsParams({ q: 'markets', gl: 'usa' }),
        /gl must be a two-letter code/,
    );
    assert.throws(
        () => buildGoogleNewsParams({ q: 'markets', hl: 123 }),
        /hl must be a string/,
    );
});

test('describes requests for logs', () => {
    assert.equal(
        describeGoogleNewsRequest({ q: 'markets', page: 2 }),
        'query "markets" (page 2)',
    );
    assert.equal(
        describeGoogleNewsRequest({ story_token: 'CAAqExample', start: 10 }),
        'story_token CAAqExample (start 10)',
    );
});
