import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildIndeedFallbackParams,
    transformIndeedFallbackResponse,
} from '../dist/jobs-fallback.js';

test('builds Indeed fallback params from a Google Jobs query with location', () => {
    assert.deepEqual(
        buildIndeedFallbackParams({
            q: 'nurse jobs in Austin',
            gl: 'us',
            hl: 'en',
            google_domain: 'google.com',
        }),
        {
            query: 'nurse',
            location: 'Austin',
            country: 'US',
            gl: 'us',
            hl: 'en',
            limit: 10,
        }
    );
});

test('keeps the full query when no location phrase is present', () => {
    assert.deepEqual(
        buildIndeedFallbackParams({ q: 'remote product manager', gl: 'de' }),
        {
            query: 'remote product manager',
            country: 'DE',
            gl: 'de',
            limit: 10,
        }
    );
});

test('transforms Indeed fallback results to Google Jobs dataset shape', () => {
    const response = transformIndeedFallbackResponse(
        {
            success: true,
            data: {
                jobs: [
                    {
                        id: 'abc123',
                        title: 'Registered Nurse',
                        company: { name: 'Example Health' },
                        location: { formatted: 'Austin, TX' },
                        description_html: '<p>Care for patients &amp; coordinate shifts.</p>',
                        date_published: '2026-05-01',
                        attributes: ['Full-time', 'Day shift'],
                        apply_url: 'https://example.com/apply',
                    },
                ],
                pagination: { next_cursor: 'cursor' },
                metadata: { source: 'indeed' },
            },
        },
        { q: 'nurse jobs in Austin' },
        'Scrappa API error (504): Gateway Timeout'
    );

    assert.equal(response.service_used, 'indeed');
    assert.equal(response.fallback_from, 'google_jobs');
    assert.equal(response.search_information?.total_results, 1);
    assert.deepEqual(response.pagination, { next_cursor: 'cursor' });
    assert.equal(response.jobs_results?.[0].title, 'Registered Nurse');
    assert.equal(response.jobs_results?.[0].company_name, 'Example Health');
    assert.equal(response.jobs_results?.[0].via, 'Indeed');
    assert.equal(response.jobs_results?.[0].description, 'Care for patients & coordinate shifts.');
    assert.deepEqual(response.jobs_results?.[0].extensions, ['2026-05-01', 'Full-time', 'Day shift']);
});
