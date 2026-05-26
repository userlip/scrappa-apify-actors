import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildErrorDatasetItem,
    buildSuccessDatasetItem,
    extractScrappaStatusCode,
    patentPageUrl,
} from '../dist/response-utils.js';

const context = {
    inputPatentId: 'US9789384B1',
    normalizedPatentId: 'patent/US9789384B1/en',
};

test('builds Google Patents page URLs from normalized IDs', () => {
    assert.equal(
        patentPageUrl('patent/US9789384B1/en'),
        'https://patents.google.com/patent/US9789384B1',
    );
});

test('builds success dataset items with flattened counts', () => {
    assert.deepEqual(
        buildSuccessDatasetItem({
            success: true,
            data: {
                patent_id: 'patent/US9789384B1/en',
                publication_number: 'US9789384B1',
                title: 'Self-balancing board having a suspension interface',
                abstract: 'Patent abstract',
                inventors: ['Jane Doe', 'John Doe'],
                assignees: {
                    original: ['Example Inc.', 'Example Labs'],
                    current: ['Example Holdings'],
                },
                country: 'US',
                language: 'en',
                application_number: 'US14/123456',
                prior_art_keywords: ['self-balancing', 'suspension'],
                links: { pdf: 'https://example.com/patent.pdf' },
                citations: {
                    patent: [{ publication_number: 'US1' }],
                    non_patent: [{ title: 'Paper' }],
                },
                cached: false,
                response_time_ms: 1234,
            },
        }, context),
        {
            input_patent_id: 'US9789384B1',
            normalized_patent_id: 'patent/US9789384B1/en',
            success: true,
            patent_id: 'patent/US9789384B1/en',
            publication_number: 'US9789384B1',
            patent_page: 'https://patents.google.com/patent/US9789384B1',
            title: 'Self-balancing board having a suspension interface',
            abstract: 'Patent abstract',
            inventors: ['Jane Doe', 'John Doe'],
            assignees: {
                original: ['Example Inc.', 'Example Labs'],
                current: ['Example Holdings'],
            },
            country: 'US',
            language: 'en',
            application_number: 'US14/123456',
            prior_art_keywords: ['self-balancing', 'suspension'],
            links: { pdf: 'https://example.com/patent.pdf' },
            citations: {
                patent: [{ publication_number: 'US1' }],
                non_patent: [{ title: 'Paper' }],
            },
            inventor_count: 2,
            assignee_count: 3,
            citation_count: 2,
            cached: false,
            response_time_ms: 1234,
        },
    );
});

test('builds error dataset items with status code extraction', () => {
    assert.deepEqual(
        buildErrorDatasetItem(new Error('Scrappa API error (404): Patent not found'), context),
        {
            input_patent_id: 'US9789384B1',
            normalized_patent_id: 'patent/US9789384B1/en',
            success: false,
            patent_id: null,
            publication_number: null,
            patent_page: 'https://patents.google.com/patent/US9789384B1',
            title: null,
            abstract: null,
            country: null,
            language: null,
            application_number: null,
            prior_art_keywords: [],
            links: {},
            citations: {},
            inventor_count: 0,
            assignee_count: 0,
            citation_count: 0,
            cached: null,
            response_time_ms: null,
            error: 'Scrappa API error (404): Patent not found',
            status_code: 404,
        },
    );
});

test('builds error dataset items from string errors', () => {
    assert.deepEqual(
        buildErrorDatasetItem('something failed', context),
        {
            input_patent_id: 'US9789384B1',
            normalized_patent_id: 'patent/US9789384B1/en',
            success: false,
            patent_id: null,
            publication_number: null,
            patent_page: 'https://patents.google.com/patent/US9789384B1',
            title: null,
            abstract: null,
            country: null,
            language: null,
            application_number: null,
            prior_art_keywords: [],
            links: {},
            citations: {},
            inventor_count: 0,
            assignee_count: 0,
            citation_count: 0,
            cached: null,
            response_time_ms: null,
            error: 'something failed',
            status_code: null,
        },
    );
});

test('extracts Scrappa status codes', () => {
    assert.equal(extractScrappaStatusCode('Scrappa API error (500): Failed'), 500);
    assert.equal(extractScrappaStatusCode('Network failed'), null);
});

test('returns null for malformed patent page IDs', () => {
    assert.equal(patentPageUrl('US9789384B1'), null);
});
