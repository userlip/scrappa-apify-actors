import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildArbeitsagenturJobsParams,
    DEFAULT_ARBEITSAGENTUR_JOBS_INPUT,
    DEFAULT_ARBEITSAGENTUR_JOBS_QUERY,
    normalizeArbeitsagenturJobsInput,
} from '../dist/arbeitsagentur-params.js';

test('forwards Arbeitsagentur jobs search parameters', () => {
    assert.deepEqual(
        buildArbeitsagenturJobsParams({
            was: 'Software Entwickler',
            wo: 'Berlin',
            berufsfeld: 'IT',
            arbeitgeber: 'TechGmbH',
            angebotsart: 1,
            arbeitszeit: 'vz;ho',
            befristung: 2,
            veroeffentlichtseit: 7,
            umkreis: 25,
            zeitarbeit: false,
            pav: true,
            page: 2,
            size: 25,
        }),
        {
            was: 'Software Entwickler',
            wo: 'Berlin',
            berufsfeld: 'IT',
            arbeitgeber: 'TechGmbH',
            angebotsart: 1,
            arbeitszeit: 'vz;ho',
            befristung: 2,
            veroeffentlichtseit: 7,
            umkreis: 25,
            zeitarbeit: false,
            pav: true,
            page: 2,
            size: 25,
        }
    );
});

test('filters undefined values from Arbeitsagentur jobs parameters', () => {
    assert.deepEqual(buildArbeitsagenturJobsParams({ was: undefined, wo: undefined, size: undefined }), {});
});

test('preserves false boolean filters in Arbeitsagentur jobs parameters', () => {
    assert.deepEqual(buildArbeitsagenturJobsParams({ zeitarbeit: false, pav: false }), {
        zeitarbeit: false,
        pav: false,
    });
});

test('uses the default search when input is empty', () => {
    assert.deepEqual(normalizeArbeitsagenturJobsInput({}), DEFAULT_ARBEITSAGENTUR_JOBS_INPUT);
});

test('uses the default search when input is missing', () => {
    assert.deepEqual(normalizeArbeitsagenturJobsInput(), DEFAULT_ARBEITSAGENTUR_JOBS_INPUT);
});

test('uses the default search when input only contains unknown placeholder fields', () => {
    assert.deepEqual(normalizeArbeitsagenturJobsInput({ helloWorld: 123 }), DEFAULT_ARBEITSAGENTUR_JOBS_INPUT);
});

test('adds the default query to partial targeting input', () => {
    assert.deepEqual(normalizeArbeitsagenturJobsInput({ wo: 'Hamburg', umkreis: 50 }), {
        ...DEFAULT_ARBEITSAGENTUR_JOBS_INPUT,
        wo: 'Hamburg',
        umkreis: 50,
    });
});

test('trims empty strings, normalizes arbeitszeit, and preserves false boolean filters', () => {
    assert.deepEqual(normalizeArbeitsagenturJobsInput({ was: '   ', arbeitszeit: ' VZ ; HO ', zeitarbeit: false }), {
        ...DEFAULT_ARBEITSAGENTUR_JOBS_INPUT,
        arbeitszeit: 'vz;ho',
        zeitarbeit: false,
    });
});

assert.equal(DEFAULT_ARBEITSAGENTUR_JOBS_QUERY, DEFAULT_ARBEITSAGENTUR_JOBS_INPUT.was);
