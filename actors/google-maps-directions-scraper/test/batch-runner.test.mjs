import assert from 'node:assert/strict';
import test from 'node:test';

const modulePath = process.env.TEST_SOURCE === 'src'
    ? '../src/batch-runner.ts'
    : '../dist/batch-runner.js';
const { runDirectionsBatch } = await import(modulePath);

const requests = [
    { origin: 'A', destination: 'B', mode: 'walking', hl: 'en', params: { origin: 'A', destination: 'B', mode: 'walking', hl: 'en' }, index: 0 },
    { origin: 'C', destination: 'D', mode: 'driving', hl: 'en', params: { origin: 'C', destination: 'D', mode: 'driving', hl: 'en' }, index: 1 },
];

function writer() {
    const rows = [];
    return {
        rows,
        canSave() { return true; },
        async save(item) {
            rows.push(item);
            return { saved: true, chargedCount: 1, chargeLimitReached: false };
        },
    };
}

test('continues after one route failure and accounts for alternatives and charges', async () => {
    const calls = [];
    const client = {
        async get(endpoint, params, options) {
            calls.push({ endpoint, params, options });
            if (params.origin === 'A') {
                throw new Error('temporary route failure');
            }
            return { status: 'OK', directions: [{ distance: 50 }, { distance: 60 }] };
        },
    };
    const output = writer();
    const result = await runDirectionsBatch(requests, client, output);

    assert.equal(calls.length, 2);
    assert.equal(calls[0].endpoint, '/google-maps-directions');
    assert.equal(calls[0].options.attempts, 3);
    assert.equal(result.requested, 2);
    assert.equal(result.succeeded, 1);
    assert.equal(result.failed, 1);
    assert.equal(result.alternativesSaved, 2);
    assert.equal(result.charged, 2);
    assert.equal(output.rows.length, 2);
    assert.equal(result.failures[0].requestIndex, 0);
});

test('stops at a charge refusal without charging later rows', async () => {
    const output = {
        rows: [],
        canSave() { return true; },
        async save(item) {
            this.rows.push(item);
            return { saved: false, chargedCount: 0, chargeLimitReached: true };
        },
    };
    let calls = 0;
    const result = await runDirectionsBatch(requests, {
        async get() {
            calls += 1;
            return { status: 'OK', directions: [{ distance: 10 }] };
        },
    }, output);

    assert.equal(calls, 1);
    assert.equal(result.chargeLimitReached, true);
    assert.equal(result.alternativesSaved, 0);
    assert.equal(result.charged, 0);
});
