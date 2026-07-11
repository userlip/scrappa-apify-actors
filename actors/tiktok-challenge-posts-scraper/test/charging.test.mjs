import assert from 'node:assert/strict';
import test from 'node:test';
import { getAvailableCapacity, pushVideos, RESULT_EVENT } from '../dist/charging.js';

function paidActor(results, capacity = 10) {
    const pushed = [];
    return {
        pushed,
        getChargingManager: () => ({
            getPricingInfo: () => ({ isPayPerEvent: true }),
            calculateMaxEventChargeCountWithinLimit: () => capacity,
        }),
        async pushData(item, eventName) {
            pushed.push({ item, eventName });
            return results.shift();
        },
    };
}

test('uses remaining paid-event capacity', () => {
    assert.equal(getAvailableCapacity(paidActor([], 3), 5), 3);
});

test('stops at the charge limit and counts only charged rows', async () => {
    const actor = paidActor([
        { chargedCount: 1, eventChargeLimitReached: false },
        { chargedCount: 0, eventChargeLimitReached: true },
    ]);
    const result = await pushVideos(actor, [{ video_id: '1' }, { video_id: '2' }, { video_id: '3' }]);

    assert.deepEqual(result, { saved: 1, limitReached: true });
    assert.equal(actor.pushed.length, 2);
    assert.ok(actor.pushed.every(({ eventName }) => eventName === RESULT_EVENT));
});

test('handles cumulative chargedCount values as one saved row per call', async () => {
    const actor = paidActor([
        { chargedCount: 4, eventChargeLimitReached: false },
        { chargedCount: 5, eventChargeLimitReached: false },
    ]);
    assert.deepEqual(await pushVideos(actor, [{ video_id: '1' }, { video_id: '2' }]), { saved: 2, limitReached: false });
});
