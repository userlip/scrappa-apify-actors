import assert from 'node:assert/strict';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const runnerModule = process.env.TEST_SOURCE === 'src'
    ? '../src/run-user-items.ts'
    : '../dist/run-user-items.js';
const { buildVintedUserItemsPlan } = await import(requestParamsModule);
const { runVintedUserItems } = await import(runnerModule);

function createDataset(pushResult = { chargedCount: 1, eventChargeLimitReached: false }) {
    return {
        pushed: [],
        async pushData(items, eventName) {
            this.pushed.push({ items, eventName });
            return pushResult;
        },
    };
}

test('fetches batched sellers and pushes one dataset item per listing', async () => {
    const requests = [];
    const client = {
        async get(endpoint, params, options) {
            requests.push({ endpoint, params, options });
            return {
                items: [{ id: `${params.user_id}-${params.page}`, title: `Item ${params.user_id}` }],
                pagination: { has_next_page: false, total_pages: params.page },
            };
        },
    };
    const dataset = createDataset({ chargedCount: 1, eventChargeLimitReached: false });
    const plan = buildVintedUserItemsPlan({
        user_ids: ['111', '222'],
        country: 'DE',
        per_page: 10,
        max_pages: 2,
    });

    const summary = await runVintedUserItems({
        client,
        dataset,
        plan,
        isPayPerEvent: true,
        chargeEventName: 'user-item-result',
        attempts: 3,
    });

    assert.equal(summary.pagesFetched, 2);
    assert.equal(summary.savedItems, 2);
    assert.equal(summary.statusMessage, null);
    assert.deepEqual(requests.map((request) => request.params.user_id), ['111', '222']);
    assert.deepEqual(requests.map((request) => request.endpoint), ['/vinted/user-items', '/vinted/user-items']);
    assert.deepEqual(dataset.pushed.map((push) => push.eventName), ['user-item-result', 'user-item-result']);
    assert.equal(dataset.pushed[0].items[0].input_user_id, '111');
    assert.equal(dataset.pushed[1].items[0].request_country, 'DE');
});

test('stops a seller when a page has no listings and continues the batch', async () => {
    const requests = [];
    const client = {
        async get(endpoint, params) {
            requests.push(params);
            if (params.user_id === '111') {
                return { items: [], pagination: { has_next_page: false } };
            }

            return { data: { items: [{ id: 'item-222' }], pagination: { has_next_page: false } } };
        },
    };
    const dataset = createDataset();
    const plan = buildVintedUserItemsPlan({
        user_ids: ['111', '222'],
        max_pages: 3,
    });

    const summary = await runVintedUserItems({
        client,
        dataset,
        plan,
        isPayPerEvent: false,
        chargeEventName: 'user-item-result',
        attempts: 3,
    });

    assert.equal(summary.pagesFetched, 2);
    assert.equal(summary.savedItems, 1);
    assert.deepEqual(requests.map((request) => request.user_id), ['111', '222']);
    assert.equal(dataset.pushed.length, 1);
    assert.equal(dataset.pushed[0].eventName, undefined);
});

test('respects max_pages while pagination reports additional pages', async () => {
    const requests = [];
    const client = {
        async get(endpoint, params) {
            requests.push(params);
            return {
                items: [{ id: `item-${params.page}` }],
                pagination: { has_next_page: true, total_pages: 50 },
            };
        },
    };
    const dataset = createDataset();
    const plan = buildVintedUserItemsPlan({
        user_id: '111',
        page: 2,
        max_pages: 2,
    });

    const summary = await runVintedUserItems({
        client,
        dataset,
        plan,
        isPayPerEvent: true,
        chargeEventName: 'user-item-result',
        attempts: 3,
    });

    assert.equal(summary.pagesFetched, 2);
    assert.equal(summary.savedItems, 2);
    assert.deepEqual(requests.map((request) => request.page), [2, 3]);
});

test('stops without fetching more users when the pay-per-event charge limit is reached', async () => {
    const requests = [];
    const client = {
        async get(endpoint, params) {
            requests.push(params);
            return { items: [{ id: 'one' }, { id: 'two' }] };
        },
    };
    const dataset = createDataset({ chargedCount: 1, eventChargeLimitReached: true });
    const plan = buildVintedUserItemsPlan({
        user_ids: ['111', '222'],
        max_pages: 2,
    });

    const summary = await runVintedUserItems({
        client,
        dataset,
        plan,
        isPayPerEvent: true,
        chargeEventName: 'user-item-result',
        attempts: 3,
    });

    assert.equal(summary.pagesFetched, 1);
    assert.equal(summary.savedItems, 1);
    assert.match(summary.statusMessage, /Charge limit reached after saving 1 of 2/);
    assert.deepEqual(requests.map((request) => request.user_id), ['111']);
});

test('propagates Scrappa errors without writing dataset items', async () => {
    const client = {
        async get() {
            throw new Error('Scrappa API error (400): user_id is required');
        },
    };
    const dataset = createDataset();
    const plan = buildVintedUserItemsPlan({ user_id: '111' });

    await assert.rejects(
        () => runVintedUserItems({
            client,
            dataset,
            plan,
            isPayPerEvent: true,
            chargeEventName: 'user-item-result',
            attempts: 3,
        }),
        /Scrappa API error \(400\): user_id is required/,
    );
    assert.equal(dataset.pushed.length, 0);
});
