import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildIndeedJobsParams,
    DEFAULT_INDEED_JOBS_INPUT,
    DEFAULT_INDEED_JOBS_QUERY,
    normalizeIndeedJobsInput,
} from '../dist/indeed-params.js';

test('forwards Indeed jobs search parameters', () => {
    assert.deepEqual(
        buildIndeedJobsParams({
            query: 'software engineer',
            location: 'New York',
            country: 'US',
            radius: 25,
            radius_unit: 'MILES',
            job_type: 'full_time',
            sort: 'date',
            limit: 20,
            cursor: 'next-cursor',
            hl: 'en',
            gl: 'US',
        }),
        {
            query: 'software engineer',
            location: 'New York',
            country: 'US',
            radius: 25,
            radius_unit: 'MILES',
            job_type: 'full_time',
            sort: 'date',
            limit: 20,
            cursor: 'next-cursor',
            hl: 'en',
            gl: 'US',
        }
    );
});

test('filters undefined values from Indeed jobs parameters', () => {
    assert.deepEqual(buildIndeedJobsParams({ query: undefined, location: undefined, limit: undefined }), {});
});

test('uses the default search when input is empty', () => {
    assert.deepEqual(normalizeIndeedJobsInput({}), DEFAULT_INDEED_JOBS_INPUT);
});

test('uses the default search when input is missing', () => {
    assert.deepEqual(normalizeIndeedJobsInput(), DEFAULT_INDEED_JOBS_INPUT);
});

test('uses the default search when input only contains unknown placeholder fields', () => {
    assert.deepEqual(normalizeIndeedJobsInput({ helloWorld: 123 }), DEFAULT_INDEED_JOBS_INPUT);
});

test('adds the default query to partial targeting input', () => {
    assert.deepEqual(normalizeIndeedJobsInput({ location: 'Berlin', country: 'de' }), {
        ...DEFAULT_INDEED_JOBS_INPUT,
        location: 'Berlin',
        country: 'DE',
    });
});

test('trims empty strings and normalizes locale fields', () => {
    assert.deepEqual(normalizeIndeedJobsInput({ query: '   ', hl: ' EN ', gl: ' de ', radius_unit: 'kilometers' }), {
        ...DEFAULT_INDEED_JOBS_INPUT,
        hl: 'en',
        gl: 'DE',
        radius_unit: 'KILOMETERS',
    });
});

assert.equal(DEFAULT_INDEED_JOBS_QUERY, DEFAULT_INDEED_JOBS_INPUT.query);
