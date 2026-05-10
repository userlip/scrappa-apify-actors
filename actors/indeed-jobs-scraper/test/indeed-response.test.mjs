import assert from 'node:assert/strict';
import test from 'node:test';

import {
    getCompanyName,
    getFormattedLocation,
    getIndeedJobs,
    getIndeedMetadata,
    getIndeedPagination,
} from '../dist/indeed-response.js';

test('returns jobs from wrapped Scrappa response data', () => {
    const jobs = [{ title: 'Software Engineer' }];

    assert.deepEqual(getIndeedJobs({
        success: true,
        data: { jobs },
    }), jobs);
});

test('returns jobs from top-level response shape', () => {
    const jobs = [{ title: 'Software Engineer' }];

    assert.deepEqual(getIndeedJobs({ jobs }), jobs);
});

test('falls back to top-level jobs when data exists without jobs', () => {
    const jobs = [{ title: 'Nurse' }];

    assert.deepEqual(getIndeedJobs({
        data: { metadata: { total_results: 1 } },
        jobs,
    }), jobs);
});

test('returns an empty jobs array for unexpected response shape', () => {
    const originalDebug = console.debug;
    const messages = [];
    console.debug = (message) => messages.push(message);

    try {
        assert.deepEqual(getIndeedJobs({ success: false }), []);
    } finally {
        console.debug = originalDebug;
    }

    assert.deepEqual(messages, [
        'Unexpected Indeed Jobs response shape: expected "data.jobs" or "jobs" array.',
    ]);
});

test('reads pagination and metadata from wrapped or top-level response fields', () => {
    assert.deepEqual(getIndeedPagination({ data: { pagination: { next_cursor: 'wrapped' } } }), { next_cursor: 'wrapped' });
    assert.deepEqual(getIndeedPagination({ pagination: { next_cursor: 'top-level' } }), { next_cursor: 'top-level' });
    assert.deepEqual(getIndeedMetadata({ data: { metadata: { total_results: 20 } } }), { total_results: 20 });
    assert.deepEqual(getIndeedMetadata({ metadata: { total_results: 10 } }), { total_results: 10 });
});

test('extracts company names from string or object companies', () => {
    assert.equal(getCompanyName('Example Corp'), 'Example Corp');
    assert.equal(getCompanyName({ name: 'Example Health' }), 'Example Health');
    assert.equal(getCompanyName(null), undefined);
});

test('formats Indeed locations for table summaries', () => {
    assert.equal(getFormattedLocation('Austin, TX'), 'Austin, TX');
    assert.equal(getFormattedLocation({ formatted: 'New York, NY' }), 'New York, NY');
    assert.equal(getFormattedLocation({ city: 'Berlin', country: 'DE' }), 'Berlin, DE');
    assert.equal(getFormattedLocation(null), undefined);
});
