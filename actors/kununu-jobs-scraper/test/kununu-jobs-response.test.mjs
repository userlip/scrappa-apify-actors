import assert from 'node:assert/strict';
import test from 'node:test';

import {
    getCompanyName,
    getFormattedLocation,
    getKununuJobs,
    getKununuMetadata,
    getKununuPagination,
    toKununuDatasetJob,
} from '../dist/kununu-jobs-response.js';

test('returns jobs from common Scrappa response shapes', () => {
    const jobs = [{ title: 'Software Engineer' }];

    assert.deepEqual(getKununuJobs({ data: { jobs } }), jobs);
    assert.deepEqual(getKununuJobs({ data: { results: jobs } }), jobs);
    assert.deepEqual(getKununuJobs({ data: jobs }), jobs);
    assert.deepEqual(getKununuJobs({ jobs }), jobs);
    assert.deepEqual(getKununuJobs({ results: jobs }), jobs);
});

test('throws when Scrappa returns a business-level failure', () => {
    assert.throws(
        () => getKununuJobs({ success: false, message: 'Invalid Kununu request' }),
        /Invalid Kununu request/
    );
});

test('returns an empty jobs array for unexpected non-failure response shape', () => {
    const originalWarn = console.warn;
    const messages = [];
    console.warn = (message) => messages.push(message);

    try {
        assert.deepEqual(getKununuJobs({ success: true }), []);
    } finally {
        console.warn = originalWarn;
    }

    assert.deepEqual(messages, [
        'Unexpected Kununu Jobs response shape: expected "data.jobs", "data.results", "jobs", or "results" array.',
    ]);
});

test('reads pagination and metadata from wrapped or top-level response fields', () => {
    assert.deepEqual(getKununuPagination({ data: { pagination: { page: 2 } } }), { page: 2 });
    assert.deepEqual(getKununuPagination({ meta: { pagination: { page: 3 } } }), { page: 3 });
    assert.deepEqual(getKununuMetadata({ data: { metadata: { country: 'de' } } }), { country: 'de' });
    assert.deepEqual(getKununuMetadata({ metadata: { country: 'at' } }), { country: 'at' });
});

test('extracts company names from string, object, or top-level job fields', () => {
    assert.equal(getCompanyName('Example GmbH'), 'Example GmbH');
    assert.equal(getCompanyName({ name: 'Example Health' }), 'Example Health');
    assert.equal(getCompanyName(null, { company_name: 'Fallback AG' }), 'Fallback AG');
});

test('formats Kununu locations for table summaries', () => {
    assert.equal(getFormattedLocation('Berlin'), 'Berlin');
    assert.equal(getFormattedLocation({ formatted: 'Zurich' }), 'Zurich');
    assert.equal(getFormattedLocation({ city: 'Vienna', region: 'Vienna', country: 'AT' }), 'Vienna, Vienna, AT');
    assert.equal(getFormattedLocation(null, { city: 'Berlin', region: 'Berlin', stateCode: 'DE-BE', countryCode: 'DE' }), 'Berlin, Berlin, DE');
    assert.equal(getFormattedLocation(null), undefined);
});

test('adds table-friendly dataset aliases and optional raw job payload', () => {
    const job = {
        id: 'job-1',
        title: 'Software Engineer',
        url: 'https://www.kununu.com/de/example/jobs/job-1',
        company: {
            name: 'Example GmbH',
            slug: 'example',
            url: 'https://www.kununu.com/de/example',
            kununu_score: 4.4,
            is_top_company: true,
        },
        location: { formatted: 'Berlin', city: 'Berlin', region: null, country: 'DE' },
    };

    assert.deepEqual(toKununuDatasetJob(job, { includeRawJob: true }), {
        title: 'Software Engineer',
        job_id: 'job-1',
        job_url: 'https://www.kununu.com/de/example/jobs/job-1',
        company: {
            name: 'Example GmbH',
            slug: 'example',
            url: 'https://www.kununu.com/de/example',
            kununu_score: 4.4,
            is_top_company: true,
        },
        company_name: 'Example GmbH',
        company_slug: 'example',
        company_url: 'https://www.kununu.com/de/example',
        company_score: 4.4,
        company_is_top_company: true,
        location_formatted: 'Berlin',
        location_city: 'Berlin',
        location_region: null,
        location_country: 'DE',
        employment_types: null,
        date_posted: null,
        posted_at: null,
        raw_job: job,
    });
});

test('normalizes live Kununu camelCase job fields', () => {
    const job = {
        id: '5ffd841a-edbd-43c5-b923-89df0e02534b',
        title: 'Senior Software Engineer',
        url: 'https://www.kununu.com/job-postings/de/5ffd841a-edbd-43c5-b923-89df0e02534b',
        postedAt: '2026-07-03',
        city: 'Berlin',
        region: 'Berlin',
        stateCode: 'DE-BE',
        employmentTypes: ['JOB_EMPLOYMENT_FULLTIME'],
        company: {
            name: 'FindYou Consulting GmbH',
            slug: 'findyou-consulting',
            website: 'https://www.findyou.de',
            score: 5,
            isTopCompany: true,
        },
    };

    assert.deepEqual(toKununuDatasetJob(job), {
        ...job,
        job_id: '5ffd841a-edbd-43c5-b923-89df0e02534b',
        job_url: 'https://www.kununu.com/job-postings/de/5ffd841a-edbd-43c5-b923-89df0e02534b',
        company_name: 'FindYou Consulting GmbH',
        company_slug: 'findyou-consulting',
        company_url: 'https://www.findyou.de',
        company_score: 5,
        company_is_top_company: true,
        location_formatted: 'Berlin, Berlin',
        location_city: 'Berlin',
        location_region: 'Berlin',
        location_country: null,
        employment_types: ['JOB_EMPLOYMENT_FULLTIME'],
        date_posted: '2026-07-03',
        posted_at: '2026-07-03',
    });
});

test('merges partial nested locations with top-level job location fields', () => {
    const job = {
        id: 'job-2',
        title: 'Frontend Engineer',
        city: 'Berlin',
        region: 'Berlin',
        countryCode: 'DE',
        stateCode: 'DE-BE',
        location: { city: null, region: null, country: null },
    };

    assert.deepEqual(toKununuDatasetJob(job), {
        ...job,
        job_id: 'job-2',
        job_url: null,
        company: null,
        company_name: null,
        company_slug: null,
        company_url: null,
        company_score: null,
        company_is_top_company: null,
        location_formatted: 'Berlin, Berlin, DE',
        location_city: 'Berlin',
        location_region: 'Berlin',
        location_country: 'DE',
        employment_types: null,
        date_posted: null,
        posted_at: null,
    });
});
