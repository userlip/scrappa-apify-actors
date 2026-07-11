import assert from 'node:assert/strict';
import test from 'node:test';
import { saveChallengeDetail } from '../dist/charged-save.js';

const item = { challenge_id: '1' };

test('saves without an event for development pricing', async () => {
    const calls = [];
    const result = await saveChallengeDetail(item, {
        getPricingInfo: () => ({ isPayPerEvent: false }),
    }, {
        pushData: async (...args) => { calls.push(args); },
    }, 'challenge-detail-result');

    assert.deepEqual(result, { savedCount: 1, chargeLimitReached: false });
    assert.deepEqual(calls, [[item]]);
});

test('charges exactly one successful pay-per-event dataset item', async () => {
    const calls = [];
    const result = await saveChallengeDetail(item, {
        getPricingInfo: () => ({ isPayPerEvent: true }),
    }, {
        pushData: async (...args) => {
            calls.push(args);
            return { chargedCount: 1, eventChargeLimitReached: false };
        },
    }, 'challenge-detail-result');

    assert.deepEqual(result, { savedCount: 1, chargeLimitReached: false });
    assert.deepEqual(calls, [[item, 'challenge-detail-result']]);
});
