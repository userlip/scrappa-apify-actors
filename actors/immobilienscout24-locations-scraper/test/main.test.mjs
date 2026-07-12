import assert from 'node:assert/strict';
import test from 'node:test';

const modulePath = process.env.TEST_SOURCE === 'src' ? '../src/main.ts' : '../dist/main.js';
const { processLocationRequests } = await import(modulePath);

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

test('stops processing when the charge limit is reached', async () => {
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
    assert.deepEqual(calls, ['Berlin']);
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
