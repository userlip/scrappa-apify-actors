import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildLinkedInJobsSearchParams,
    DEFAULT_LINKEDIN_JOBS_SEARCH_INPUT,
    DEFAULT_LINKEDIN_JOBS_SEARCH_QUERY,
    normalizeLinkedInJobsSearchInput,
} from '../dist/search-params.js';

test('forwards LinkedIn jobs search parameters', () => {
    assert.deepEqual(
        buildLinkedInJobsSearchParams({
            query: 'software engineer berlin',
            num: 20,
            page: 2,
            hl: 'en',
            lr: 'lang_en',
            gl: 'us',
            cr: 'countryUS',
            safe: 'off',
            dateRestrict: 'm1',
            sort: 'date',
            filter: 1,
            rights: 'cc_publicdomain',
        }),
        {
            query: 'software engineer berlin',
            num: 20,
            page: 2,
            hl: 'en',
            lr: 'lang_en',
            gl: 'us',
            cr: 'countryUS',
            safe: 'off',
            dateRestrict: 'm1',
            sort: 'date',
            filter: 1,
            rights: 'cc_publicdomain',
        }
    );
});

test('filters undefined values from LinkedIn jobs search parameters', () => {
    assert.deepEqual(buildLinkedInJobsSearchParams({ query: 'cto', start: undefined, gl: undefined }), {
        query: 'cto',
    });
});

test('uses the default query when input is empty', () => {
    assert.deepEqual(normalizeLinkedInJobsSearchInput({}), DEFAULT_LINKEDIN_JOBS_SEARCH_INPUT);
});

test('uses the default query when input is missing', () => {
    assert.deepEqual(normalizeLinkedInJobsSearchInput(), DEFAULT_LINKEDIN_JOBS_SEARCH_INPUT);
});

test('uses the default query when input only contains unknown placeholder fields', () => {
    assert.deepEqual(normalizeLinkedInJobsSearchInput({ helloWorld: 123 }), DEFAULT_LINKEDIN_JOBS_SEARCH_INPUT);
});

test('adds the default query to partial targeting input', () => {
    assert.deepEqual(normalizeLinkedInJobsSearchInput({ gl: 'de' }), {
        ...DEFAULT_LINKEDIN_JOBS_SEARCH_INPUT,
        gl: 'de',
    });
});

test('trims empty strings before applying defaults', () => {
    assert.deepEqual(normalizeLinkedInJobsSearchInput({ query: '   ', hl: ' de ' }), {
        ...DEFAULT_LINKEDIN_JOBS_SEARCH_INPUT,
        hl: 'de',
    });
});

test('preserves explicit query and applies default targeting', () => {
    assert.deepEqual(normalizeLinkedInJobsSearchInput({ query: ' nurse jobs Austin ', num: 5 }), {
        ...DEFAULT_LINKEDIN_JOBS_SEARCH_INPUT,
        query: 'nurse jobs Austin',
        num: 5,
    });
});

assert.equal(DEFAULT_LINKEDIN_JOBS_SEARCH_QUERY, DEFAULT_LINKEDIN_JOBS_SEARCH_INPUT.query);
