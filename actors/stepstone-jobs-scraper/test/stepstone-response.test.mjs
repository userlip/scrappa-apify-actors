import assert from 'node:assert/strict';
import test from 'node:test';

import {
    getCompanyName,
    getCompanyUrl,
    getFormattedLocation,
    getStepstoneJobs,
    getStepstoneMetadata,
    getStepstonePagination,
    toStepstoneDatasetJob,
} from '../dist/stepstone-response.js';

test('returns jobs from wrapped Scrappa response data', () => {
    const jobs = [{ title: 'Software Engineer' }];

    assert.deepEqual(getStepstoneJobs({
        success: true,
        data: { jobs },
    }), jobs);
});

test('returns jobs from top-level response shape', () => {
    const jobs = [{ title: 'Software Engineer' }];

    assert.deepEqual(getStepstoneJobs({ jobs }), jobs);
});

test('falls back to top-level jobs when data exists without jobs', () => {
    const jobs = [{ title: 'Nurse' }];

    assert.deepEqual(getStepstoneJobs({
        data: { metadata: { total_jobs: 1 } },
        jobs,
    }), jobs);
});

test('returns an empty jobs array for unexpected response shape', () => {
    const originalDebug = console.debug;
    const messages = [];
    console.debug = (message) => messages.push(message);

    try {
        assert.deepEqual(getStepstoneJobs({ success: false }), []);
    } finally {
        console.debug = originalDebug;
    }

    assert.deepEqual(messages, [
        'Unexpected Stepstone Jobs response shape: expected "data.jobs" or "jobs" array.',
    ]);
});

test('reads pagination and metadata from wrapped or top-level response fields', () => {
    assert.deepEqual(getStepstonePagination({ data: { pagination: { next_page: 2 } } }), { next_page: 2 });
    assert.deepEqual(getStepstonePagination({ pagination: { next_page: 3 } }), { next_page: 3 });
    assert.deepEqual(getStepstoneMetadata({ data: { metadata: { country: 'de' } } }), { country: 'de' });
    assert.deepEqual(getStepstoneMetadata({ metadata: { country: 'at' } }), { country: 'at' });
});

test('extracts company names and URLs from string or object companies', () => {
    assert.equal(getCompanyName('Example GmbH'), 'Example GmbH');
    assert.equal(getCompanyName({ name: 'Example Health' }), 'Example Health');
    assert.equal(getCompanyName(null), undefined);
    assert.equal(getCompanyUrl({ url: 'https://www.stepstone.de/cmp/example' }), 'https://www.stepstone.de/cmp/example');
    assert.equal(getCompanyUrl('Example GmbH'), undefined);
});

test('formats Stepstone locations for table summaries', () => {
    assert.equal(getFormattedLocation('Berlin'), 'Berlin');
    assert.equal(getFormattedLocation({ formatted: 'Amsterdam' }), 'Amsterdam');
    assert.equal(getFormattedLocation({ city: 'Brussels', region: 'Brussels-Capital', country: 'BE' }), 'Brussels, Brussels-Capital, BE');
    assert.equal(getFormattedLocation(null), undefined);
});

test('adds table-friendly dataset aliases while preserving raw job fields', () => {
    const job = {
        title: 'Software Engineer',
        company: { name: 'Example GmbH', url: 'https://www.stepstone.de/cmp/example' },
        location: { formatted: 'Berlin', city: 'Berlin', region: null, country: 'DE' },
    };

    assert.deepEqual(toStepstoneDatasetJob(job), {
        ...job,
        company_name: 'Example GmbH',
        company_url: 'https://www.stepstone.de/cmp/example',
        location_formatted: 'Berlin',
        location_city: 'Berlin',
        location_region: null,
        location_country: 'DE',
    });
});
