import assert from 'node:assert/strict';
import test from 'node:test';
import { RESULT_EVENT } from '../dist/charging.js';
import { scrapeChallenge } from '../dist/scrape.js';

function actor(capacity = Infinity) {
    const rows = [];
    const events = [];
    return {
        rows,
        events,
        getChargingManager: () => ({
            getPricingInfo: () => ({ isPayPerEvent: true }),
            calculateMaxEventChargeCountWithinLimit: () => capacity - rows.length,
        }),
        async pushData(data, event) {
            const items = Array.isArray(data) ? data : [data];
            const saved = items.slice(0, Math.max(0, capacity - rows.length));
            rows.push(...saved);
            events.push(...saved.map(() => event));
            return { chargedCount: rows.length, eventChargeLimitReached: saved.length < items.length || rows.length >= capacity };
        },
    };
}

const request = { challengeId: '123', region: 'US', resultLimit: 10, pageSize: 2 };

test('follows pagination, deduplicates video IDs, and keeps event/dataset parity', async () => {
    const calls = [];
    const client = { async get(_path, params) {
        calls.push(params);
        return calls.length === 1
            ? { code: 0, data: { videos: [{ video_id: '1' }, { video_id: '2' }], cursor: 2, hasMore: true } }
            : { code: 0, data: { videos: [{ video_id: '2' }, { video_id: '3' }], cursor: 4, hasMore: false } };
    } };
    const paidActor = actor();
    const result = await scrapeChallenge(client, paidActor, request);

    assert.equal(result.status, 'succeeded');
    assert.equal(result.videos_saved, 3);
    assert.equal(calls.length, 2);
    assert.deepEqual(paidActor.rows.map((row) => row.video_id), ['1', '2', '3']);
    assert.deepEqual(paidActor.events, [RESULT_EVENT, RESULT_EVENT, RESULT_EVENT]);
});

test('checks charge capacity before fetching another page', async () => {
    let calls = 0;
    const client = { async get(_path, params) {
        calls += 1;
        assert.equal(params.count, 2);
        return { code: 0, data: { videos: [{ video_id: '1' }, { video_id: '2' }], cursor: 2, hasMore: true } };
    } };
    const result = await scrapeChallenge(client, actor(2), request);
    assert.equal(result.status, 'charge-limit-reached');
    assert.equal(calls, 1);
    assert.equal(result.videos_saved, 2);
});

test('reduces upstream page size to available paid capacity', async () => {
    const client = { async get(_path, params) {
        assert.equal(params.count, 1);
        return { code: 0, data: { videos: [{ video_id: '1' }], cursor: 1, hasMore: true } };
    } };
    const result = await scrapeChallenge(client, actor(1), request);
    assert.equal(result.status, 'charge-limit-reached');
    assert.equal(result.videos_saved, 1);
});

test('returns a partial failure summary without throwing', async () => {
    const client = { async get() { return { code: 429, msg: 'rate limited' }; } };
    const result = await scrapeChallenge(client, actor(), request);
    assert.equal(result.status, 'failed');
    assert.match(result.error, /rate limited/);
});

test('continues past duplicate-only pages while the cursor advances', async () => {
    let calls = 0;
    const client = { async get() {
        calls += 1;
        if (calls === 1) return { code: 0, data: { videos: [{ video_id: '1' }], cursor: 1, hasMore: true } };
        if (calls === 2) return { code: 0, data: { videos: [{ video_id: '1' }], cursor: 2, hasMore: true } };
        return { code: 0, data: { videos: [{ video_id: '2' }], cursor: 3, hasMore: false } };
    } };
    const paidActor = actor();
    const result = await scrapeChallenge(client, paidActor, { ...request, resultLimit: 2, pageSize: 1 });

    assert.equal(result.videos_saved, 2);
    assert.equal(calls, 3);
    assert.deepEqual(paidActor.rows.map((row) => row.video_id), ['1', '2']);
});
