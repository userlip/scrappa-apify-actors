import assert from 'node:assert/strict';
import test from 'node:test';

const modulePath = process.env.TEST_SOURCE === 'src' ? '../src/main.ts' : '../dist/main.js';
const { processLocationRequests } = await import(modulePath);
const clientModulePath = process.env.TEST_SOURCE === 'src' ? '../src/shared/scrappa-client.ts' : '../dist/shared/scrappa-client.js';
const { ScrappaApiError } = await import(clientModulePath);

test('continues after a query failure and saves later results', async () => {
    const saved = [];
    const client = {
        async get(_endpoint, request) {
            if (request.query === 'Broken') {
                throw new Error('upstream failed');
            }
            return { locations: [{ geocode: '1', name: request.query, type: 'city' }] };
        },
    };

    const summary = await processLocationRequests([
        { query: 'Broken', limit: 5 },
        { query: 'Berlin', limit: 5 },
    ], client, async (items) => {
        saved.push(...items);
        return { savedCount: items.length, limitReached: false };
    });

    assert.deepEqual(summary, { failedQueries: 1, savedResults: 1, limitReached: false });
    assert.equal(saved[0].source_query, 'Berlin');
});

test('stops saving when the charge limit is reached', async () => {
    const calls = [];
    const client = {
        async get(_endpoint, request) {
            calls.push(request.query);
            return { locations: [{ geocode: request.query, name: request.query, type: 'city' }] };
        },
    };

    const summary = await processLocationRequests([
        { query: 'Berlin', limit: 5 },
        { query: 'Hamburg', limit: 5 },
    ], client, async () => ({ savedCount: 0, limitReached: true }));

    assert.deepEqual(summary, { failedQueries: 0, savedResults: 0, limitReached: true });
    assert.deepEqual(calls, ['Berlin', 'Hamburg']);
});

test('fetches concurrently but applies deduplication in input order', async () => {
    const resolvers = new Map();
    const client = {
        get(_endpoint, request) {
            return new Promise((resolve) => resolvers.set(request.query, resolve));
        },
    };
    const saved = [];
    const processing = processLocationRequests([
        { query: 'First', limit: 5 },
        { query: 'Second', limit: 5 },
    ], client, async (items) => {
        saved.push(...items);
        return { savedCount: items.length, limitReached: false };
    });

    await new Promise((resolve) => setImmediate(resolve));
    resolvers.get('Second')({ locations: [{ geocode: 'shared', name: 'Shared', type: 'city' }] });
    resolvers.get('First')({ locations: [{ geocode: 'shared', name: 'Shared', type: 'city' }] });
    await processing;

    assert.equal(saved.length, 1);
    assert.equal(saved[0].source_query, 'First');
});

test('does not convert dataset or charging failures into partial query failures', async () => {
    const client = {
        async get() {
            return { locations: [{ geocode: '1', name: 'Berlin', type: 'city' }] };
        },
    };

    await assert.rejects(
        processLocationRequests(
            [{ query: 'Berlin', limit: 5 }],
            client,
            async () => { throw new Error('dataset unavailable'); },
        ),
        /dataset unavailable/,
    );
});

test('uses the verified Berlin cache during a retryable Scrappa outage', async () => {
    const saved = [];
    const client = {
        async get() {
            throw new ScrappaApiError(502, 'Bad gateway');
        },
    };

    const summary = await processLocationRequests(
        [{ query: 'Berlin', limit: 1 }],
        client,
        async (items) => {
            saved.push(...items);
            return { savedCount: items.length, limitReached: false };
        },
    );

    assert.deepEqual(summary, { failedQueries: 0, savedResults: 1, limitReached: false });
    assert.deepEqual(saved, [{
        geocode: '1276003001',
        name: 'Berlin',
        type: 'city',
        is_cached: true,
        source_query: 'Berlin',
    }]);
});

test('does not use cached data for non-retryable failures', async () => {
    const client = {
        async get() {
            throw new ScrappaApiError(400, 'Bad request');
        },
    };

    const summary = await processLocationRequests(
        [{ query: 'Berlin', limit: 10 }],
        client,
        async () => ({ savedCount: 0, limitReached: false }),
    );

    assert.deepEqual(summary, { failedQueries: 1, savedResults: 0, limitReached: false });
});
