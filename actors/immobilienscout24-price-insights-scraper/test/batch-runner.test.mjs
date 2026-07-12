import assert from 'node:assert/strict';
import test from 'node:test';
const batchRunnerPath = process.env.TEST_SOURCE === 'src' ? '../src/batch-runner.ts' : '../dist/batch-runner.js';
const sharedPath = process.env.TEST_SOURCE === 'src' ? '../src/shared/index.ts' : '../dist/shared/index.js';
const { PRICE_INSIGHT_RESULT_EVENT, runPriceInsightsBatch } = await import(batchRunnerPath);
const { ScrappaApiError } = await import(sharedPath);

const response = (location) => ({
    success: true,
    location,
    geocode: '123',
    currency: 'EUR',
    prices: {
        apartment_rent_per_m2: 10,
        apartment_buy_per_m2: 4000,
        house_rent_per_m2: 12,
        house_buy_per_m2: 4200,
    },
});

test('continues after a failed location and charges only successful snapshots', async () => {
    const calls = [];
    const client = {
        async get(_endpoint, params) {
            if (params.location === 'Nowhere') throw new ScrappaApiError(404, 'Location not found');
            return response(params.location);
        },
    };
    const writer = {
        isPayPerEvent: () => true,
        async pushData(item, eventName) {
            calls.push({ item, eventName });
            return { chargedCount: 1, eventChargeLimitReached: false };
        },
    };

    const result = await runPriceInsightsBatch([
        { location: 'Berlin', index: 0 },
        { location: 'Nowhere', index: 1 },
        { location: 'Munich', index: 2 },
    ], client, writer);

    assert.equal(result.succeeded, 2);
    assert.deepEqual(result.failures, [{ location: 'Nowhere', message: 'Location not found', status: 404 }]);
    assert.equal(calls.length, 2);
    assert.ok(calls.every((call) => call.eventName === PRICE_INSIGHT_RESULT_EVENT));
});

test('writes successful snapshots without an event on non-PPE builds', async () => {
    const calls = [];
    const result = await runPriceInsightsBatch(
        [{ location: 'Berlin', index: 0 }],
        { get: async () => response('Berlin') },
        {
            isPayPerEvent: () => false,
            async pushData(item, eventName) {
                calls.push({ item, eventName });
                return { chargedCount: 0, eventChargeLimitReached: false };
            },
        },
    );

    assert.equal(result.succeeded, 1);
    assert.equal(calls[0].eventName, undefined);
});

test('stops writing after a charged result reaches the event limit', async () => {
    const fetched = [];
    let writes = 0;
    const result = await runPriceInsightsBatch(
        [{ location: 'Berlin', index: 0 }, { location: 'Munich', index: 1 }],
        {
            async get(_endpoint, params) {
                fetched.push(params.location);
                return response(params.location);
            },
        },
        {
            isPayPerEvent: () => true,
            async pushData() {
                writes += 1;
                return { chargedCount: 1, eventChargeLimitReached: true };
            },
        },
    );

    assert.deepEqual(result, { succeeded: 1, failures: [], chargeLimitReached: true });
    assert.deepEqual(fetched, ['Berlin', 'Munich']);
    assert.equal(writes, 1);
});

test('does not convert dataset or charging failures into location failures', async () => {
    await assert.rejects(
        runPriceInsightsBatch(
            [{ location: 'Berlin', index: 0 }],
            { get: async () => response('Berlin') },
            {
                isPayPerEvent: () => true,
                async pushData() {
                    throw new Error('dataset unavailable');
                },
            },
        ),
        /dataset unavailable/,
    );
});

test('fetches concurrently but writes results in input order', async () => {
    const resolvers = new Map();
    const writes = [];
    const processing = runPriceInsightsBatch(
        [{ location: 'Berlin', index: 0 }, { location: 'Munich', index: 1 }],
        {
            get(_endpoint, params) {
                return new Promise((resolve) => resolvers.set(params.location, resolve));
            },
        },
        {
            isPayPerEvent: () => false,
            async pushData(item) {
                writes.push(item.request_location);
                return { chargedCount: 0, eventChargeLimitReached: false };
            },
        },
    );

    await new Promise((resolve) => setImmediate(resolve));
    resolvers.get('Munich')(response('Munich'));
    resolvers.get('Berlin')(response('Berlin'));
    await processing;

    assert.deepEqual(writes, ['Berlin', 'Munich']);
});
