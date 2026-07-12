import assert from 'node:assert/strict';
import test from 'node:test';

const modulePath = process.env.TEST_SOURCE === 'src' ? '../src/input.ts' : '../dist/input.js';
const { buildLocationRequests } = await import(modulePath);

test('normalizes and case-insensitively deduplicates batch queries', () => {
    assert.deepEqual(buildLocationRequests({
        queries: [' Berlin ', 'berlin', ' Hamburg '],
        query: 'ignored',
        limit: 5,
    }), [
        { query: 'Berlin', limit: 5 },
        { query: 'Hamburg', limit: 5 },
    ]);
});

test('supports singular query compatibility and the default upstream limit', () => {
    assert.deepEqual(buildLocationRequests({ query: ' 10115 ' }), [
        { query: '10115', limit: 10 },
    ]);
});

test('validates input shape, empty queries, query count, and upstream limit range', () => {
    assert.throws(() => buildLocationRequests({}), /Provide at least one/);
    assert.throws(() => buildLocationRequests({ queries: 'Berlin' }), /must be an array/);
    assert.throws(() => buildLocationRequests({ queries: [' '] }), /must not be empty/);
    assert.throws(() => buildLocationRequests({ queries: Array.from({ length: 101 }, (_, index) => `q${index}`) }), /at most 100/);
    assert.throws(() => buildLocationRequests({ query: 'Berlin', limit: 0 }), /between 1 and 20/);
    assert.throws(() => buildLocationRequests({ query: 'Berlin', limit: 21 }), /between 1 and 20/);
    assert.throws(() => buildLocationRequests({ query: 'Berlin', limit: 1.5 }), /must be an integer/);
});
