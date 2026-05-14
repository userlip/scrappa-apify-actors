import assert from 'node:assert/strict';
import test from 'node:test';

import { buildGooglePatentsSearchParams, describeGooglePatentsSearchRequest } from '../dist/request-params.js';

test('builds params for a filtered patent search', () => {
    assert.deepEqual(
        buildGooglePatentsSearchParams({
            q: ' wireless charging ',
            page: 2,
            num: 25,
            sort: 'NEW',
            before: 'FILING:20231231',
            after: 'publication:20200101',
            country: ' US, EP , WO ',
            language: ' ENGLISH ',
            status: 'grant',
            type: 'patent',
            inventor: 'Ada Lovelace, Grace Hopper',
            assignee: 'Tesla, Toyota',
        }),
        {
            q: 'wireless charging',
            page: 2,
            num: 25,
            sort: 'new',
            before: 'filing:20231231',
            after: 'publication:20200101',
            country: 'US,EP,WO',
            language: 'ENGLISH',
            status: 'GRANT',
            type: 'PATENT',
            inventor: 'Ada Lovelace,Grace Hopper',
            assignee: 'Tesla,Toyota',
        },
    );
});

test('requires a query', () => {
    assert.throws(
        () => buildGooglePatentsSearchParams({ page: 1 }),
        /q is required/,
    );
});

test('rejects invalid pagination and result counts', () => {
    assert.throws(
        () => buildGooglePatentsSearchParams({ q: 'battery', page: 0 }),
        /page must be between 1 and 100/,
    );
    assert.throws(
        () => buildGooglePatentsSearchParams({ q: 'battery', num: 101 }),
        /num must be between 1 and 100/,
    );
});

test('rejects invalid enums', () => {
    assert.throws(
        () => buildGooglePatentsSearchParams({ q: 'battery', sort: 'relevance' }),
        /sort must be one of/,
    );
    assert.throws(
        () => buildGooglePatentsSearchParams({ q: 'battery', status: 'expired' }),
        /status must be one of/,
    );
    assert.throws(
        () => buildGooglePatentsSearchParams({ q: 'battery', type: 'plant' }),
        /type must be one of/,
    );
});

test('rejects invalid date filters', () => {
    assert.throws(
        () => buildGooglePatentsSearchParams({ q: 'battery', before: 'created:20230101' }),
        /before must use format/,
    );
    assert.throws(
        () => buildGooglePatentsSearchParams({ q: 'battery', after: 'filing:20230230' }),
        /after must include a valid calendar date/,
    );
});

test('describes requests for logs', () => {
    assert.equal(
        describeGooglePatentsSearchRequest({
            q: 'battery',
            page: 2,
            num: 25,
            sort: 'new',
            country: 'US',
            status: 'GRANT',
        }),
        'query "battery" page 2 (25 results per page) sorted by new with country=US, status=GRANT',
    );
});
