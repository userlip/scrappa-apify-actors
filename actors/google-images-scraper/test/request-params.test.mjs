import assert from 'node:assert/strict';
import test from 'node:test';

import { buildGoogleImagesParamList, buildGoogleImagesParams, describeGoogleImagesRequest } from '../dist/request-params.js';

test('builds params for a filtered image search', () => {
    assert.deepEqual(
        buildGoogleImagesParams({
            q: ' coffee product photography ',
            page: 2,
            hl: 'EN',
            gl: 'US',
            imgsz: 'LARGE',
            imgtype: 'photo',
            imgcolor: 'white',
            imgar: 'wide',
            tbs: ' qdr:w ',
            safe: 'ACTIVE',
        }),
        {
            q: 'coffee product photography',
            page: 2,
            hl: 'en',
            gl: 'us',
            imgsz: 'large',
            imgtype: 'photo',
            imgcolor: 'white',
            imgar: 'wide',
            tbs: 'qdr:w',
            safe: 'active',
        },
    );
});

test('builds params for multiple image searches and deduplicates queries', () => {
    assert.deepEqual(
        buildGoogleImagesParamList({
            q: 'coffee product photography',
            queries: [' coffee product photography ', 'espresso machine'],
            gl: 'US',
            safe: 'ACTIVE',
        }),
        [
            {
                q: 'coffee product photography',
                gl: 'us',
                safe: 'active',
            },
            {
                q: 'espresso machine',
                gl: 'us',
                safe: 'active',
            },
        ],
    );
});

test('validates image query batch input shape', () => {
    assert.throws(
        () => buildGoogleImagesParamList({ queries: 'coffee' }),
        /queries must be an array/,
    );
    assert.throws(
        () => buildGoogleImagesParamList({
            q: 'extra query',
            queries: Array.from({ length: 50 }, (_, index) => `query ${index}`),
        }),
        /queries must contain 50 items or fewer/,
    );
});

test('requires a non-empty query', () => {
    assert.throws(
        () => buildGoogleImagesParams({ q: '   ' }),
        /q is required/,
    );
});

test('rejects invalid localization and pagination controls', () => {
    assert.throws(
        () => buildGoogleImagesParams({ q: 'coffee', page: 0 }),
        /page must be greater than or equal to 1/,
    );
    assert.throws(
        () => buildGoogleImagesParams({ q: 'coffee', hl: 'eng' }),
        /hl must be 2 characters or fewer/,
    );
    assert.throws(
        () => buildGoogleImagesParams({ q: 'coffee', gl: 123 }),
        /gl must be a string/,
    );
});

test('rejects invalid filter values', () => {
    assert.throws(
        () => buildGoogleImagesParams({ q: 'coffee', imgsz: 'huge' }),
        /imgsz must be one of: large, medium, icon/,
    );
    assert.throws(
        () => buildGoogleImagesParams({ q: 'coffee', imgtype: 'svg' }),
        /imgtype must be one of: photo, clipart, lineart, gif, face/,
    );
    assert.throws(
        () => buildGoogleImagesParams({ q: 'coffee', imgcolor: 'cyan' }),
        /imgcolor must be one of:/,
    );
    assert.throws(
        () => buildGoogleImagesParams({ q: 'coffee', imgar: 'panoramic' }),
        /imgar must be one of: tall, square, wide/,
    );
    assert.throws(
        () => buildGoogleImagesParams({ q: 'coffee', safe: 'moderate' }),
        /safe must be one of: active, off/,
    );
});

test('describes requests for logs', () => {
    assert.equal(
        describeGoogleImagesRequest({ q: 'coffee', page: 2, imgsz: 'large', safe: 'active' }),
        'query "coffee" page 2 (imgsz=large, safe=active)',
    );
});
