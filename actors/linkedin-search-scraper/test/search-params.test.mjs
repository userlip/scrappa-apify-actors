import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildLinkedInSearchParams,
    DEFAULT_LINKEDIN_SEARCH_INPUT,
    DEFAULT_LINKEDIN_SEARCH_QUERY,
    limitLinkedInSearchResultCount,
    normalizeLinkedInSearchInput,
    validateLinkedInSearchInput,
} from '../dist/search-params.js';

test('forwards LinkedIn search parameters', () => {
    assert.deepEqual(
        buildLinkedInSearchParams({
            query: 'site:linkedin.com/in founder AI Berlin',
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
            query: 'site:linkedin.com/in founder AI Berlin',
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

test('filters undefined values from LinkedIn search parameters', () => {
    assert.deepEqual(buildLinkedInSearchParams({ query: 'cto', start: undefined, gl: undefined }), {
        query: 'cto',
    });
});

test('uses the default query when input is empty', () => {
    assert.deepEqual(normalizeLinkedInSearchInput({}), DEFAULT_LINKEDIN_SEARCH_INPUT);
});

test('uses the default query when input is missing', () => {
    assert.deepEqual(normalizeLinkedInSearchInput(), DEFAULT_LINKEDIN_SEARCH_INPUT);
});

test('uses the default query when input only contains unknown placeholder fields', () => {
    assert.deepEqual(normalizeLinkedInSearchInput({ helloWorld: 123 }), DEFAULT_LINKEDIN_SEARCH_INPUT);
});

test('adds the default query to partial targeting input', () => {
    assert.deepEqual(normalizeLinkedInSearchInput({ gl: 'de' }), {
        ...DEFAULT_LINKEDIN_SEARCH_INPUT,
        gl: 'de',
    });
});

test('trims empty strings before applying defaults', () => {
    assert.deepEqual(normalizeLinkedInSearchInput({ query: '   ', hl: ' de ' }), {
        ...DEFAULT_LINKEDIN_SEARCH_INPUT,
        hl: 'de',
    });
});

test('preserves explicit query and applies default targeting', () => {
    assert.deepEqual(normalizeLinkedInSearchInput({ query: ' site:linkedin.com/company fintech Berlin ', num: 5 }), {
        ...DEFAULT_LINKEDIN_SEARCH_INPUT,
        query: 'site:linkedin.com/company fintech Berlin',
        num: 5,
    });
});

test('rejects page and start together', () => {
    assert.throws(
        () => validateLinkedInSearchInput({ query: 'site:linkedin.com/in founder', page: 1, start: 0 }),
        /Use either page or start/,
    );
});

test('rejects invalid numeric ranges', () => {
    assert.throws(
        () => validateLinkedInSearchInput({ query: 'site:linkedin.com/in founder', num: 21 }),
        /num must be an integer from 1 to 20/,
    );
    assert.throws(
        () => validateLinkedInSearchInput({ query: 'site:linkedin.com/in founder', page: 11 }),
        /page must be an integer from 1 to 10/,
    );
    assert.throws(
        () => validateLinkedInSearchInput({ query: 'site:linkedin.com/in founder', start: 171 }),
        /start must be an integer from 0 to 170/,
    );
});

test('accepts valid normalized search input', () => {
    assert.doesNotThrow(() => validateLinkedInSearchInput({
        query: 'site:linkedin.com/in founder AI Berlin',
        num: 3,
        gl: 'de',
        hl: 'en',
    }));
});

test('caps result count to remaining charge capacity', () => {
    assert.deepEqual(
        limitLinkedInSearchResultCount({ query: 'site:linkedin.com/in founder', num: 10 }, 3),
        { query: 'site:linkedin.com/in founder', num: 3 },
    );
});

test('keeps result count unchanged outside pay-per-event charge limits', () => {
    const input = { query: 'site:linkedin.com/in founder', num: 10 };

    assert.equal(limitLinkedInSearchResultCount(input, null), input);
});

test('uses the default result count when charge-capping input without num', () => {
    assert.deepEqual(
        limitLinkedInSearchResultCount({ query: 'site:linkedin.com/in founder' }, 3),
        { query: 'site:linkedin.com/in founder', num: 3 },
    );
});

assert.equal(DEFAULT_LINKEDIN_SEARCH_QUERY, DEFAULT_LINKEDIN_SEARCH_INPUT.query);
