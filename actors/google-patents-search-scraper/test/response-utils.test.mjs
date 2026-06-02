import assert from 'node:assert/strict';
import test from 'node:test';

import {
    enrichResult,
    extractPatentResults,
    extractPatentSearchData,
    limitPatentSearchResponse,
    patentPageUrl,
} from '../dist/response-utils.js';

test('extracts patent data from wrapped and direct responses', () => {
    const data = { patents: [{ patent_id: 'patent/US123B1/en' }], total_results: 1 };

    assert.deepEqual(extractPatentSearchData({ success: true, data }), data);
    assert.deepEqual(extractPatentSearchData(data), data);
    assert.deepEqual(extractPatentResults({ success: true, data }), data.patents);
    assert.deepEqual(extractPatentResults(data), data.patents);
});

test('returns empty result set for missing patents array', () => {
    const originalWarn = console.warn;
    const warnings = [];
    console.warn = (message) => warnings.push(message);

    try {
        assert.deepEqual(extractPatentResults({ success: true, data: { total_results: 0 } }), []);
        assert.deepEqual(warnings, ['Scrappa Google Patents response did not include a patents result array']);
    } finally {
        console.warn = originalWarn;
    }
});

test('builds patent page URLs', () => {
    assert.equal(patentPageUrl({ publication_number: 'US123B1' }), 'https://patents.google.com/patent/US123B1');
    assert.equal(patentPageUrl({ patent_id: 'patent/EP123A1/en' }), 'https://patents.google.com/patent/EP123A1');
    assert.equal(patentPageUrl({ patent_id: 'EP123A1' }), null);
    assert.equal(patentPageUrl({ patent_id: 'patent/EP123A1' }), null);
    assert.equal(patentPageUrl({ patent_id: 'patent/EP123A1/eng' }), null);
    assert.equal(patentPageUrl({}), null);
});

test('enriches patent results with flattened fields, request metadata, and upstream fields', () => {
    assert.deepEqual(
        enrichResult(
            {
                patent_id: 'patent/US123B1/en',
                rank: 1,
                title: 'Charging system',
                upstream_score: 92,
                dates: {
                    priority: '2020-01-01',
                    filing: '2021-01-01',
                    grant: '2024-01-01',
                    publication: '2022-01-01',
                },
                family_status: [
                    { country: 'US', status: 'ACTIVE' },
                    { country: 'EP', status: 'PENDING' },
                ],
            },
            {
                q: 'charging',
                page: 1,
                num: 10,
                status: 'GRANT',
            },
        ),
        {
            patent_id: 'patent/US123B1/en',
            rank: 1,
            title: 'Charging system',
            upstream_score: 92,
            dates: {
                priority: '2020-01-01',
                filing: '2021-01-01',
                grant: '2024-01-01',
                publication: '2022-01-01',
            },
            family_status: [
                { country: 'US', status: 'ACTIVE' },
                { country: 'EP', status: 'PENDING' },
            ],
            patent_page: 'https://patents.google.com/patent/US123B1',
            snippet: null,
            publication_number: null,
            language: null,
            priority_date: '2020-01-01',
            filing_date: '2021-01-01',
            grant_date: '2024-01-01',
            publication_date: '2022-01-01',
            inventor: null,
            assignee: null,
            thumbnail: null,
            pdf: null,
            family_status_count: 2,
            family_countries: 'US,EP',
            request_q: 'charging',
            request_page: 1,
            request_num: 10,
            request_sort: null,
            request_before: null,
            request_after: null,
            request_country: null,
            request_language: null,
            request_status: 'GRANT',
            request_type: null,
            request_inventor: null,
            request_assignee: null,
        },
    );
});

test('enriches null patent page and empty family countries for boundary patent IDs', () => {
    const result = enrichResult(
        {
            patent_id: 'patent/US123B1/eng',
            family_status: [],
        },
        { q: 'charging' },
    );

    assert.equal(result.patent_page, null);
    assert.equal(result.family_status_count, 0);
    assert.equal(result.family_countries, null);
});

test('limits patent payloads to the requested result count', () => {
    const patents = [
        { patent_id: 'patent/US1/en' },
        { patent_id: 'patent/US2/en' },
        { patent_id: 'patent/US3/en' },
    ];

    assert.deepEqual(
        limitPatentSearchResponse({ success: true, data: { patents, total_results: 3 }, trace_id: 'abc' }, 2),
        { success: true, data: { patents: patents.slice(0, 2), total_results: 3 }, trace_id: 'abc' },
    );

    assert.deepEqual(
        limitPatentSearchResponse({ patents, total_results: 3 }, 1),
        { patents: patents.slice(0, 1), total_results: 3 },
    );

    assert.deepEqual(
        limitPatentSearchResponse({ success: true, data: { patents: [], total_results: 0 } }, 0),
        { success: true, data: { patents: [], total_results: 0 } },
    );
});
