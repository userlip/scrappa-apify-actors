import assert from 'node:assert/strict';
import test from 'node:test';

import { buildStartpageSearchPlan, describeStartpageSearchPlan } from '../dist/request-params.js';

test('builds batch Startpage request params', () => {
    assert.deepEqual(
        buildStartpageSearchPlan({
            queries: [
                {
                    query: ' privacy tools ',
                    language: 'ENGLISH',
                    page: 0,
                    safe_search: true,
                },
                {
                    query: ' private search ',
                    safe_search: false,
                },
            ],
            max_results_per_query: 10,
        }),
        {
            requests: [
                {
                    query: 'privacy tools',
                    params: {
                        query: 'privacy tools',
                        language: 'english',
                        page: 0,
                        safe_search: 1,
                    },
                },
                {
                    query: 'private search',
                    params: {
                        query: 'private search',
                        safe_search: 0,
                    },
                },
            ],
            maxResultsPerQuery: 10,
        },
    );
});

test('omits safe_search when it is not provided', () => {
    assert.deepEqual(
        buildStartpageSearchPlan({
            queries: [{ query: 'privacy tools' }],
        }),
        {
            requests: [
                {
                    query: 'privacy tools',
                    params: { query: 'privacy tools' },
                },
            ],
            maxResultsPerQuery: 20,
        },
    );
});

test('rejects invalid queries and controls', () => {
    assert.throws(() => buildStartpageSearchPlan({}), /queries must be an array/);
    assert.throws(() => buildStartpageSearchPlan({ queries: [] }), /at least one/);
    assert.throws(() => buildStartpageSearchPlan({ queries: ['privacy'] }), /queries\[0\] must be an object/);
    assert.throws(() => buildStartpageSearchPlan({ queries: [{ query: '   ' }] }), /queries\[0\]\.query is required/);
    assert.throws(() => buildStartpageSearchPlan({ queries: [{ query: 'privacy', language: 'klingon' }] }), /language must be one of/);
    assert.throws(() => buildStartpageSearchPlan({ queries: [{ query: 'privacy', page: 11 }] }), /page must be between 0 and 10/);
    assert.throws(() => buildStartpageSearchPlan({ queries: [{ query: 'privacy', safe_search: 'false' }] }), /safe_search must be a boolean/);
    assert.throws(() => buildStartpageSearchPlan({ queries: [{ query: 'privacy' }], max_results_per_query: 0 }), /max_results_per_query must be between 1 and 100/);
});

test('describes batch requests for logs', () => {
    assert.equal(
        describeStartpageSearchPlan(buildStartpageSearchPlan({
            queries: [
                { query: 'one' },
                { query: 'two' },
                { query: 'three' },
                { query: 'four' },
            ],
        })),
        '4 query request(s): "one", "two", "three" and 1 more',
    );
});
