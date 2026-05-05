import assert from 'node:assert/strict';
import test from 'node:test';

import { getJobs, getNextPageToken } from '../dist/jobs-response.js';

test('uses jobs when the primary jobs array has results', () => {
    const jobs = [{ title: 'Primary job' }];

    assert.deepEqual(getJobs({
        jobs,
        jobs_results: [{ title: 'Fallback job' }],
    }), jobs);
});

test('falls back to jobs_results when jobs is empty', () => {
    const jobsResults = [{ title: 'Fallback job' }];

    assert.deepEqual(getJobs({
        jobs: [],
        jobs_results: jobsResults,
    }), jobsResults);
});

test('returns an empty jobs array when no fallback results exist', () => {
    assert.deepEqual(getJobs({ jobs: [] }), []);
    assert.deepEqual(getJobs({}), []);
});

test('reads next page token from top-level or pagination response fields', () => {
    assert.equal(getNextPageToken({ next_page_token: 'top-level-token' }), 'top-level-token');
    assert.equal(getNextPageToken({ pagination: { next_page_token: 'pagination-token' } }), 'pagination-token');
    assert.equal(getNextPageToken({}), undefined);
});
