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
                assignees: { current: ['Example Inc.'] },
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
            assignees: { current: ['Example Inc.'] },
            citations: {
                patent: [{ publication_number: 'US1' }],
                non_patent: [{ title: 'Paper' }],
            },
            inventor_count: 2,
            assignee_count: 1,
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
            error: 'Scrappa API error (404): Patent not found',
            status_code: 404,
        },
    );
});

test('extracts Scrappa status codes', () => {
    assert.equal(extractScrappaStatusCode('Scrappa API error (500): Failed'), 500);
    assert.equal(extractScrappaStatusCode('Network failed'), null);
});
