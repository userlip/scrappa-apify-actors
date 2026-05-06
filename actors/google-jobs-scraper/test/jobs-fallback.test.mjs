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
    assert.equal(response.jobs_results?.[0].link, 'https://example.com/apply');
    assert.equal(response.jobs_results?.[0].share_link, undefined);
    assert.deepEqual(response.jobs_results?.[0].extensions, ['2026-05-01', 'Full-time', 'Day shift']);
});

test('transforms top-level Indeed fallback response shape', () => {
    const response = transformIndeedFallbackResponse(
        {
            jobs: [
                {
                    id: 'top-level-job',
                    title: 'Clinic RN',
                    company: 'Example Clinic',
                    location: 'Austin, TX',
                    description: 'Direct patient care',
                },
            ],
        },
        { q: 'rn jobs in Austin' },
        'Scrappa API error (502): Bad Gateway'
    );

    assert.equal(response.search_information?.total_results, 1);
    assert.equal(response.jobs_results?.[0].job_id, 'top-level-job');
    assert.equal(response.jobs_results?.[0].company_name, 'Example Clinic');
    assert.equal(response.jobs_results?.[0].description, 'Direct patient care');
});

test('preserves fallback pagination without advertising an unsupported next page token', () => {
    const response = transformIndeedFallbackResponse(
        {
            data: {
                jobs: [],
                pagination: {
                    next_cursor: 'indeed-cursor',
                    page: 1,
                },
            },
        },
        { q: 'rn jobs in Austin' },
        'Scrappa API error (504): Gateway Timeout'
    );

    assert.equal(response.next_page_token, undefined);
    assert.deepEqual(response.pagination, {
        next_cursor: 'indeed-cursor',
        page: 1,
    });
});

test('returns empty fallback results for non-object responses', () => {
    const response = transformIndeedFallbackResponse(null, { q: 'nurse jobs in Austin' }, 'timeout');

    assert.deepEqual(response.jobs_results, []);
    assert.equal(response.search_information?.total_results, 0);
    assert.equal(response.fallback_reason, 'timeout');
});

test('treats unsuccessful Indeed wrapper without data as empty fallback response', () => {
    const response = transformIndeedFallbackResponse(
        { success: false, message: 'upstream failed' },
        { q: 'nurse jobs in Austin' },
        'timeout'
    );

    assert.deepEqual(response.jobs_results, []);
    assert.equal(response.metadata, undefined);
});

test('strips HTML and decodes common entities in fallback descriptions', () => {
    const response = transformIndeedFallbackResponse(
        {
            jobs: [
                {
                    title: 'Nurse',
                    description_html: '<div>Use &lt;care&gt; &amp; coordinate &quot;shifts&quot; &apos;daily&apos;.</div>',
                },
            ],
        },
        { q: 'nurse jobs' },
        'timeout'
    );

    assert.equal(response.jobs_results?.[0].description, 'Use <care> & coordinate "shifts" \'daily\'.');
});

test('prefers plain fallback descriptions over HTML descriptions', () => {
    const response = transformIndeedFallbackResponse(
        {
            jobs: [
                {
                    title: 'Nurse',
                    description: 'Plain description wins.',
                    description_html: '<p>HTML description loses.</p>',
                },
            ],
        },
        { q: 'nurse jobs' },
        'timeout'
    );

    assert.equal(response.jobs_results?.[0].description, 'Plain description wins.');
});

test('filters empty and non-string fallback attributes from extensions', () => {
    const response = transformIndeedFallbackResponse(
        {
            jobs: [
                {
                    title: 'Nurse',
                    date_published: '',
                    attributes: ['Full-time', '', null, 42, 'Day shift'],
                },
            ],
        },
        { q: 'nurse jobs' },
        'timeout'
    );

    assert.deepEqual(response.jobs_results?.[0].extensions, ['Full-time', 'Day shift']);
});
