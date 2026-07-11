import assert from 'node:assert/strict';
import test from 'node:test';
import { runChallengeDetailsBatch } from '../dist/batch-runner.js';

const requests = [
    { type: 'challenge_name', value: 'missing', params: { challenge_name: 'missing' } },
    { type: 'challenge_id', value: '1', params: { challenge_id: '1' } },
];

test('preserves a later success after an upstream failure and only saves successes', async () => {
    const saved = [];
    const summary = await runChallengeDetailsBatch(requests, {
        getCapacity: () => Infinity,
        fetch: async (request) => request.value === 'missing' ? { code: 404, msg: 'Not found' } : { data: { id: '1', challenge_name: 'one' } },
        save: async (item) => { saved.push(item); return { savedCount: 1, chargeLimitReached: false }; },
    });
    assert.equal(summary.attempted, 2); assert.equal(summary.saved, 1); assert.equal(summary.failed, 1);
    assert.equal(saved.length, 1); assert.match(summary.outcomes[0].error, /404/);
});

test('stops before a request when charge capacity is exhausted', async () => {
    let fetched = 0;
    const summary = await runChallengeDetailsBatch(requests, {
        getCapacity: () => 0,
        fetch: async () => { fetched += 1; return { data: { id: '1' } }; },
        save: async () => ({ savedCount: 1, chargeLimitReached: false }),
    });
    assert.equal(fetched, 0); assert.equal(summary.attempted, 0); assert.equal(summary.charge_limit_reached, true);
});

test('reports a short event charge as failed and stops after an event charge limit', async () => {
    const shortCharge = await runChallengeDetailsBatch([requests[1]], {
        getCapacity: () => 1, fetch: async () => ({ data: { id: '1' } }),
        save: async () => ({ savedCount: 0, chargeLimitReached: true }),
    });
    assert.equal(shortCharge.saved, 0); assert.equal(shortCharge.failed, 1);

    const limitAfterSave = await runChallengeDetailsBatch(requests.slice(1), {
        getCapacity: () => 1, fetch: async () => ({ data: { id: '1' } }),
        save: async () => ({ savedCount: 1, chargeLimitReached: true }),
    });
    assert.equal(limitAfterSave.saved, 1); assert.equal(limitAfterSave.charge_limit_reached, true);
});
