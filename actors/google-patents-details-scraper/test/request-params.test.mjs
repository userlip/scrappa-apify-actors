import assert from 'node:assert/strict';
import test from 'node:test';

import {
    collectGooglePatentsDetailsRequests,
    describeGooglePatentsDetailsRequest,
    normalizeGooglePatentIdentifier,
} from '../dist/request-params.js';

test('normalizes short patent publication IDs', () => {
    assert.equal(
        normalizeGooglePatentIdentifier(' us9789384b1 '),
        'patent/US9789384B1/en',
    );
});

test('normalizes full Google Patents IDs', () => {
    assert.equal(
        normalizeGooglePatentIdentifier('patent/ep3892147a1/de'),
        'patent/EP3892147A1/de',
    );
});

test('extracts publication IDs from Google Patents URLs', () => {
    assert.equal(
        normalizeGooglePatentIdentifier('https://patents.google.com/patent/US9789384B1?oq=US9789384B1'),
        'patent/US9789384B1/en',
    );
    assert.equal(
        normalizeGooglePatentIdentifier('https://patents.google.com/patent/EP3892147A1/de'),
        'patent/EP3892147A1/de',
    );
});

test('collects, normalizes, and deduplicates batch inputs', () => {
    assert.deepEqual(
        collectGooglePatentsDetailsRequests({
            patent_id: 'US9789384B1',
            patent_ids: ['EP3892147A1', 'us9789384b1'],
            url: 'https://patents.google.com/patent/WO2020123456A1',
            urls: ['https://patents.google.com/patent/EP3892147A1'],
        }),
        [
            {
                inputPatentId: 'US9789384B1',
                normalizedPatentId: 'patent/US9789384B1/en',
                params: { patent_id: 'patent/US9789384B1/en' },
            },
            {
                inputPatentId: 'EP3892147A1',
                normalizedPatentId: 'patent/EP3892147A1/en',
                params: { patent_id: 'patent/EP3892147A1/en' },
            },
            {
                inputPatentId: 'https://patents.google.com/patent/WO2020123456A1',
                normalizedPatentId: 'patent/WO2020123456A1/en',
                params: { patent_id: 'patent/WO2020123456A1/en' },
            },
        ],
    );
});

test('deduplicates short IDs and URLs for the same patent', () => {
    assert.deepEqual(
        collectGooglePatentsDetailsRequests({
            patent_id: 'US9789384B1',
            urls: ['https://patents.google.com/patent/US9789384B1'],
        }),
        [
            {
                inputPatentId: 'US9789384B1',
                normalizedPatentId: 'patent/US9789384B1/en',
                params: { patent_id: 'patent/US9789384B1/en' },
            },
        ],
    );
});

test('treats empty arrays as missing input', () => {
    assert.throws(
        () => collectGooglePatentsDetailsRequests({ patent_ids: [], urls: [] }),
        /At least one patent ID or Google Patents URL is required/,
    );
});

test('requires at least one patent ID or URL', () => {
    assert.throws(
        () => collectGooglePatentsDetailsRequests({}),
        /At least one patent ID or Google Patents URL is required/,
    );
});

test('rejects invalid identifiers and non-Google URLs', () => {
    assert.throws(
        () => normalizeGooglePatentIdentifier('INVALID!@#'),
        /Invalid patent ID format/,
    );
    assert.throws(
        () => normalizeGooglePatentIdentifier('https://example.com/patent/US9789384B1'),
        /Invalid Google Patents URL/,
    );
});

test('describes requests for logs', () => {
    assert.equal(
        describeGooglePatentsDetailsRequest([
            {
                inputPatentId: 'US9789384B1',
                normalizedPatentId: 'patent/US9789384B1/en',
                params: { patent_id: 'patent/US9789384B1/en' },
            },
        ]),
        'patent/US9789384B1/en',
    );
    assert.equal(
        describeGooglePatentsDetailsRequest([
            {
                inputPatentId: 'US9789384B1',
                normalizedPatentId: 'patent/US9789384B1/en',
                params: { patent_id: 'patent/US9789384B1/en' },
            },
            {
                inputPatentId: 'EP3892147A1',
                normalizedPatentId: 'patent/EP3892147A1/en',
                params: { patent_id: 'patent/EP3892147A1/en' },
            },
        ]),
        '2 patents',
    );
});
