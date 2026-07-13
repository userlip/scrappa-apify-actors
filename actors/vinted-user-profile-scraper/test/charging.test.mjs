import test from 'node:test';
import assert from 'node:assert/strict';

const sourceDirectory = process.env.TEST_SOURCE === 'src' ? 'src' : 'dist';
const charging = await import(`../${sourceDirectory}/charging.js`);

function actorStub({ payPerEvent = true, remaining = 1, pushResult = { chargedCount: 1 } } = {}) {
    const calls = [];
    return {
        calls,
        getChargingManager: () => ({
            getPricingInfo: () => ({ isPayPerEvent: payPerEvent }),
            calculateMaxEventChargeCountWithinLimit: () => remaining,
        }),
        pushData: async (item, eventName) => {
            calls.push({ item, eventName });
            return pushResult;
        },
    };
}

test('uses the exact PPE event and saves one successful result', async () => {
    const actor = actorStub();
    const result = await charging.pushSuccessfulVintedUserProfile(actor, { id: 1 }, 0);
    assert.equal(charging.VINTED_USER_PROFILE_RESULT_CHARGE_EVENT, 'user-profile-result');
    assert.equal(result.saved, true);
    assert.deepEqual(actor.calls, [{ item: { id: 1 }, eventName: 'user-profile-result' }]);
});

test('checks charge capacity before fetching and reports exhausted capacity', () => {
    const actor = actorStub({ remaining: 0 });
    assert.equal(
        charging.getVintedUserProfileChargeLimitStatus(actor, 3, 3),
        'Charge limit reached before fetching Vinted user profile request 4; 3 profile result(s) were saved.',
    );
});

test('preserves unbounded PPE charge capacity', () => {
    const actor = actorStub({ remaining: Number.POSITIVE_INFINITY });
    assert.equal(charging.getVintedUserProfileAvailableChargeCount(actor), Number.POSITIVE_INFINITY);
    assert.equal(charging.getVintedUserProfileChargeLimitStatus(actor, 0, 0), null);
});

test('does not treat an uncharged PPE push as a saved result', async () => {
    const actor = actorStub({ pushResult: { chargedCount: 0, eventChargeLimitReached: true } });
    const result = await charging.pushSuccessfulVintedUserProfile(actor, { id: 1 }, 0);
    assert.equal(result.saved, false);
    assert.match(result.statusMessage, /before saving/);
});

test('reports when the final saved PPE result also reaches the charge limit', async () => {
    const actor = actorStub({ pushResult: { chargedCount: 1, eventChargeLimitReached: true } });
    const result = await charging.pushSuccessfulVintedUserProfile(actor, { id: 1 }, 0);
    assert.equal(result.saved, true);
    assert.match(result.statusMessage, /after saving/);
});
