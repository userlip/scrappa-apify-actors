import test from 'node:test';
import assert from 'node:assert/strict';

const sourceDirectory = process.env.TEST_SOURCE === 'src' ? 'src' : 'dist';
const { runVintedUserProfiles } = await import(`../${sourceDirectory}/run-user-profile.js`);

function requests(ids) {
    return ids.map((userId, index) => ({ userId, params: { user_id: userId, country: 'DE' }, index }));
}

function actorStub(maxCharges = 10) {
    let saved = 0;
    const pushes = [];
    return {
        pushes,
        getChargingManager: () => ({
            getPricingInfo: () => ({ isPayPerEvent: true }),
            calculateMaxEventChargeCountWithinLimit: () => maxCharges - saved,
        }),
        pushData: async (item, eventName) => {
            pushes.push({ item, eventName });
            saved += 1;
            return { chargedCount: 1 };
        },
    };
}

test('continues after an unresolved profile and charges only later successes', async () => {
    const actor = actorStub();
    const calls = [];
    const client = {
        async get(endpoint, params) {
            calls.push({ endpoint, params });
            if (params.user_id === 'bad') {
                return { success: false, message: 'User not found' };
            }
            if (params.user_id === 'sparse') {
                return { success: true, data: { user: { id: 42 } } };
            }
            return { success: true, data: { user: { id: Number(params.user_id), login: `user-${params.user_id}`, profile_url: `https://www.vinted.de/member/${params.user_id}` } } };
        },
    };

    // The runner receives already-normalized requests; the bad ID represents a
    // valid numeric request whose public profile cannot be resolved upstream.
    const summary = await runVintedUserProfiles({ actor, client, requests: requests(['bad', 'sparse', '255914028']), attempts: 1 });
    assert.deepEqual(summary, { requested: 3, succeeded: 1, failed: 2, statusMessage: null });
    assert.deepEqual(calls.map((call) => call.endpoint), ['/vinted/user-profile', '/vinted/user-profile', '/vinted/user-profile']);
    assert.equal(actor.pushes.length, 1);
    assert.equal(actor.pushes[0].eventName, 'user-profile-result');
    assert.equal(actor.pushes[0].item.request_user_id, '255914028');
    assert.deepEqual(actor.pushes.map(({ item }) => item.request_user_id), ['255914028']);
});

test('does not fetch after PPE capacity is exhausted', async () => {
    const actor = actorStub(1);
    let calls = 0;
    const client = {
        async get() {
            calls += 1;
            return { success: true, data: { user: { id: calls, login: `user-${calls}`, profile_url: `https://www.vinted.de/member/${calls}` } } };
        },
    };

    const summary = await runVintedUserProfiles({ actor, client, requests: requests(['1', '2']), attempts: 1 });
    assert.equal(calls, 1);
    assert.equal(summary.succeeded, 1);
    assert.match(summary.statusMessage, /before fetching/);
});
