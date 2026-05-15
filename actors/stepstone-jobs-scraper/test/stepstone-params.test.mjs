import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildStepstoneJobsParams,
    DEFAULT_STEPSTONE_JOBS_INPUT,
    DEFAULT_STEPSTONE_JOBS_QUERY,
    normalizeStepstoneJobsInput,
} from '../dist/stepstone-params.js';

test('forwards Stepstone jobs search parameters', () => {
    assert.deepEqual(
        buildStepstoneJobsParams({
            query: 'software engineer',
            location: 'Berlin',
            country: 'de',
            radius: 50,
            sort: 'date',
            job_type: 'full_time',
            work_from_home: true,
            date_posted: 7,
            page: 2,
            limit: 25,
        }),
        {
            query: 'software engineer',
            location: 'Berlin',
            country: 'de',
            radius: 50,
            sort: 'date',
            job_type: 'full_time',
            work_from_home: true,
            date_posted: 7,
            page: 2,
            limit: 25,
        }
    );
});

test('filters undefined values from Stepstone jobs parameters', () => {
    assert.deepEqual(buildStepstoneJobsParams({ query: undefined, location: undefined, limit: undefined }), {});
});

test('uses the default search when input is empty', () => {
    assert.deepEqual(normalizeStepstoneJobsInput({}), DEFAULT_STEPSTONE_JOBS_INPUT);
});

test('uses the default search when input is missing', () => {
    assert.deepEqual(normalizeStepstoneJobsInput(), DEFAULT_STEPSTONE_JOBS_INPUT);
});

test('uses the default search when input only contains unknown placeholder fields', () => {
    assert.deepEqual(normalizeStepstoneJobsInput({ helloWorld: 123 }), DEFAULT_STEPSTONE_JOBS_INPUT);
});

test('adds the default query to partial targeting input', () => {
    assert.deepEqual(normalizeStepstoneJobsInput({ location: 'Vienna', country: 'AT' }), {
        ...DEFAULT_STEPSTONE_JOBS_INPUT,
        location: 'Vienna',
        country: 'at',
    });
});

test('trims empty strings and preserves false boolean filters', () => {
    assert.deepEqual(normalizeStepstoneJobsInput({ query: '   ', country: ' DE ', work_from_home: false }), {
        ...DEFAULT_STEPSTONE_JOBS_INPUT,
        country: 'de',
        work_from_home: false,
    });
});

assert.equal(DEFAULT_STEPSTONE_JOBS_QUERY, DEFAULT_STEPSTONE_JOBS_INPUT.query);
