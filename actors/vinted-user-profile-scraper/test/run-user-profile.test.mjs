import test from 'node:test';
import assert from 'node:assert/strict';

const sourceDirectory = process.env.TEST_SOURCE === 'src' ? 'src' : 'dist';
const { runVintedUserProfiles } = await import(`../${sourceDirectory}/run-user-profile.js`);
const { MAX_USER_IDS_PER_RUN } = await import(`../${sourceDirectory}/request-params.js`);
const { PROFILE_REQUEST_CONCURRENCY } = await import(`../${sourceDirectory}/runtime-budget.js`);

function requests(ids) {
    return ids.map((userId, index) => ({ userId, params: { user_id: userId, country: 'DE' }, index }));
}

function actorStub(maxCharges = 10, pushResult = { chargedCount: 1 }) {
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
            return pushResult;
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

test('counts an uncharged result without a limit status as a failed request', async () => {
    const actor = actorStub(10, { chargedCount: 0 });
    const client = {
        async get() {
            return { success: true, data: { user: { id: 1, login: 'user-1', profile_url: 'https://www.vinted.de/member/1' } } };
        },
    };

    const summary = await runVintedUserProfiles({ actor, client, requests: requests(['1']), attempts: 1 });
    assert.deepEqual(summary, { requested: 1, succeeded: 0, failed: 1, statusMessage: null });
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

test('processes the maximum supported batch with bounded concurrency', async () => {
    const actor = actorStub(MAX_USER_IDS_PER_RUN);
    const ids = Array.from({ length: MAX_USER_IDS_PER_RUN }, (_, index) => String(index + 1));
    let calls = 0;
    let inFlight = 0;
    let maxInFlight = 0;
    const client = {
        async get(endpoint, params) {
            assert.equal(endpoint, '/vinted/user-profile');
            calls += 1;
            inFlight += 1;
            maxInFlight = Math.max(maxInFlight, inFlight);
            await new Promise((resolve) => setImmediate(resolve));
            inFlight -= 1;
            return {
                success: true,
                data: {
                    user: {
                        id: Number(params.user_id),
                        login: `user-${params.user_id}`,
                        profile_url: `https://www.vinted.de/member/${params.user_id}`,
                    },
                },
            };
        },
    };

    const summary = await runVintedUserProfiles({
        actor,
        client,
        requests: requests(ids),
        attempts: 2,
    });

    assert.deepEqual(summary, {
        requested: MAX_USER_IDS_PER_RUN,
        succeeded: MAX_USER_IDS_PER_RUN,
        failed: 0,
        statusMessage: null,
    });
    assert.equal(calls, MAX_USER_IDS_PER_RUN);
    assert.equal(actor.pushes.length, MAX_USER_IDS_PER_RUN);
    assert.ok(maxInFlight <= PROFILE_REQUEST_CONCURRENCY);
});
