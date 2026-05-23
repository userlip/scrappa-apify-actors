import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildPinterestSearchPlan,
    describePinterestSearchRequest,
} = await import(requestParamsModule);

test('builds a batch Pinterest search plan from queries', () => {
    const plan = buildPinterestSearchPlan({
        queries: [' home decor ', 'kitchen ideas'],
        limit: '25',
    });

    assert.deepEqual(plan, {
        requests: [
            {
                query: 'home decor',
                params: { query: 'home decor', limit: 25 },
            },
            {
                query: 'kitchen ideas',
                params: { query: 'kitchen ideas', limit: 25 },
            },
        ],
        limit: 25,
        bookmark: undefined,
    });
    assert.equal(describePinterestSearchRequest(plan), '2 queries (25 pins/query)');
});

test('supports compatibility query and bookmark', () => {
    const plan = buildPinterestSearchPlan({
        query: 'wedding flowers',
        limit: 10,
        bookmark: 'abc123',
    });

    assert.deepEqual(plan.requests, [
        {
            query: 'wedding flowers',
            params: {
                query: 'wedding flowers',
                limit: 10,
                bookmark: 'abc123',
            },
        },
    ]);
    assert.equal(plan.bookmark, 'abc123');
    assert.equal(describePinterestSearchRequest(plan), '"wedding flowers" (10 pins/query, with bookmark)');
});

test('deduplicates query and queries while preserving order', () => {
    const plan = buildPinterestSearchPlan({
        query: 'home decor',
        queries: ['home decor', ' Home Decor ', 'living room'],
    });

    assert.deepEqual(
        plan.requests.map((request) => request.query),
        ['home decor', 'Home Decor', 'living room'],
    );
    assert.equal(plan.limit, 50);
});

test('decodes encoded query values and keeps normal percent characters', () => {
    const plan = buildPinterestSearchPlan({
        queries: ['home%20office', '100% wool decor'],
    });

    assert.deepEqual(
        plan.requests.map((request) => request.query),
        ['home office', '100% wool decor'],
    );
});

test('validates required batch input and limit bounds', () => {
    assert.throws(
        () => buildPinterestSearchPlan({}),
        /Provide at least one Pinterest search query/,
    );
    assert.throws(
        () => buildPinterestSearchPlan({ queries: 'home decor' }),
        /queries must be an array of strings/,
    );
    assert.throws(
        () => buildPinterestSearchPlan({ query: 'home decor', limit: 251 }),
        /limit must be between 1 and 250/,
    );
    assert.throws(
        () => buildPinterestSearchPlan({ query: 'home decor', limit: 0 }),
        /limit must be between 1 and 250/,
    );
});

test('input schema matches the Pinterest search contract', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

    assert.equal(schema.properties.queries.type, 'array');
    assert.equal(schema.properties.query.type, 'string');
    assert.equal(schema.properties.limit.maximum, 250);
    assert.equal(schema.properties.bookmark.type, 'string');
    assert.equal(schema.properties.use_cache, undefined);
});
