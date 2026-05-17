import assert from 'node:assert/strict';
import test from 'node:test';

import { buildGoogleVideosParams, describeGoogleVideosRequest } from '../dist/request-params.js';

test('builds params for a localized video search', () => {
    assert.deepEqual(
        buildGoogleVideosParams({
            q: ' coffee brewing tutorial ',
            page: 2,
            hl: 'EN',
            gl: 'US',
            google_domain: ' google.com ',
            location: ' Austin, Texas ',
            tbs: ' qdr:w ',
            safe: 'OFF',
            filter: 1,
            nfpr: 0,
            lr: ' lang_en ',
        }),
        {
            q: 'coffee brewing tutorial',
            page: 2,
            hl: 'en',
            gl: 'us',
            google_domain: 'google.com',
            location: 'Austin, Texas',
            tbs: 'qdr:w',
            safe: 'off',
            filter: 1,
            nfpr: 0,
            lr: 'lang_en',
        },
    );
});

test('supports start offset pagination', () => {
    assert.deepEqual(
        buildGoogleVideosParams({
            q: 'coffee',
            start: 10,
            uule: 'encoded-location',
        }),
        {
            q: 'coffee',
            start: 10,
            uule: 'encoded-location',
        },
    );
});

test('requires a non-empty query', () => {
    assert.throws(
        () => buildGoogleVideosParams({ q: '   ' }),
        /q is required/,
    );
});

test('rejects invalid pagination and location combinations', () => {
    assert.throws(
        () => buildGoogleVideosParams({ q: 'coffee', page: 0 }),
        /page must be greater than or equal to 1/,
    );
    assert.throws(
        () => buildGoogleVideosParams({ q: 'coffee', start: -1 }),
        /start must be greater than or equal to 0/,
    );
    assert.throws(
        () => buildGoogleVideosParams({ q: 'coffee', page: 1, start: 0 }),
        /Cannot use both page and start parameters/,
    );
    assert.throws(
        () => buildGoogleVideosParams({ q: 'coffee', location: 'Austin', uule: 'encoded' }),
        /Cannot use both location and uule parameters/,
    );
});

test('rejects invalid localization and filter values', () => {
    assert.throws(
        () => buildGoogleVideosParams({ q: 'coffee', hl: 'eng' }),
        /hl must be 2 characters or fewer/,
    );
    assert.throws(
        () => buildGoogleVideosParams({ q: 'coffee', gl: 123 }),
        /gl must be a string/,
    );
    assert.throws(
        () => buildGoogleVideosParams({ q: 'coffee', safe: 'moderate' }),
        /safe must be one of: active, off/,
    );
    assert.throws(
        () => buildGoogleVideosParams({ q: 'coffee', filter: 2 }),
        /filter must be 0 or 1/,
    );
    assert.throws(
        () => buildGoogleVideosParams({ q: 'coffee', nfpr: 1.5 }),
        /nfpr must be an integer/,
    );
});

test('describes requests for logs', () => {
    assert.equal(
        describeGoogleVideosRequest({ q: 'coffee', page: 2, gl: 'us', safe: 'off' }),
        'query "coffee" (page 2, gl=us, safe=off)',
    );
});
