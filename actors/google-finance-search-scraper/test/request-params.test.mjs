import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildGoogleFinanceSearchRequests,
    describeGoogleFinanceSearchRequest,
    getMaxQueriesPerRun,
} from '../dist/request-params.js';

test('builds params for a single compatibility query', () => {
    assert.deepEqual(
        buildGoogleFinanceSearchRequests({
            q: ' AAPL ',
            hl: 'EN',
            gl: 'US',
        }),
        [
            {
                q: 'AAPL',
                hl: 'en',
                gl: 'us',
            },
        ],
    );
});

test('builds params for batch queries and prefers queries over q', () => {
    assert.deepEqual(
        buildGoogleFinanceSearchRequests({
            q: 'ignored',
            queries: [' Tesla ', 'MSFT'],
            hl: 'en',
        }),
        [
            { q: 'Tesla', hl: 'en' },
            { q: 'MSFT', hl: 'en' },
        ],
    );
});

test('rejects missing, invalid, and oversized query input', () => {
    assert.throws(
        () => buildGoogleFinanceSearchRequests({}),
        /q is required/,
    );
    assert.throws(
        () => buildGoogleFinanceSearchRequests({ queries: [] }),
        /queries must include at least one query/,
    );
    assert.throws(
        () => buildGoogleFinanceSearchRequests({ queries: ['AAPL', 123] }),
        /queries\[1\] must be a string/,
    );
    assert.throws(
        () => buildGoogleFinanceSearchRequests({ q: 'a'.repeat(256) }),
        /q must be 255 characters or fewer/,
    );
});

test('caps batch size conservatively', () => {
    const tooMany = Array.from({ length: getMaxQueriesPerRun() + 1 }, (_, index) => `query-${index}`);
    assert.throws(
        () => buildGoogleFinanceSearchRequests({ queries: tooMany }),
        /queries can include at most 25 items per run/,
    );
});

test('describes requests for logs', () => {
    assert.equal(
        describeGoogleFinanceSearchRequest({ q: 'AAPL', hl: 'en', gl: 'us' }),
        '"AAPL" (hl=en, gl=us)',
    );
    assert.equal(describeGoogleFinanceSearchRequest({ q: 'Tesla' }), '"Tesla"');
});
