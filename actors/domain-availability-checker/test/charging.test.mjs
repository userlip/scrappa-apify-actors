import assert from 'node:assert/strict';
import test from 'node:test';

import {
    DOMAIN_RESULT_CHARGE_EVENT,
    getDomainChargeLimitStatus,
    pushDomainResult,
} from '../dist/charging.js';

function createActorMock({ isPayPerEvent, limitCount = 1, pushResults = [] }) {
    const pushed = [];

    return {
        pushed,
        getChargingManager() {
            return {
                getPricingInfo() {
                    return { isPayPerEvent };
                },
                calculateMaxEventChargeCountWithinLimit(eventName) {
                    assert.equal(eventName, DOMAIN_RESULT_CHARGE_EVENT);
                    return limitCount;
                },
            };
        },
        async pushData(data, eventName) {
            pushed.push({ data, eventName });
            return pushResults.shift() ?? {
                chargedCount: eventName ? 1 : 0,
                eventChargeLimitReached: false,
            };
        },
    };
}

const successItem = {
    success: true,
    input_domain: 'example.com',
    domain: 'example.com',
    available: false,
    registered: true,
    status: 'registered',
    confidence: 'high',
    source: 'rdap',
    rdap_url: 'https://rdap.example/domain/example.com',
    rdap_status_code: 200,
    rdap_events: [],
    nameservers: [],
};

test('getDomainChargeLimitStatus is null outside pay-per-event', () => {
    const actor = createActorMock({ isPayPerEvent: false });

    assert.equal(getDomainChargeLimitStatus(actor, 0, 2), null);
});

test('getDomainChargeLimitStatus reports exhausted charge limit before fetching', () => {
    const actor = createActorMock({ isPayPerEvent: true, limitCount: 0 });

    assert.equal(
        getDomainChargeLimitStatus(actor, 1, 3),
        'Charge limit reached before fetching the next domain availability result; 1 of 3 domain(s) were processed.',
    );
});

test('pushDomainResult charges successful domain results in pay-per-event runs', async () => {
    const actor = createActorMock({ isPayPerEvent: true });

    const result = await pushDomainResult(actor, successItem);

    assert.deepEqual(result, { saved: true, statusMessage: null });
    assert.deepEqual(actor.pushed, [{ data: successItem, eventName: DOMAIN_RESULT_CHARGE_EVENT }]);
});

test('pushDomainResult does not charge failed domain items', async () => {
    const actor = createActorMock({ isPayPerEvent: true });
    const failureItem = { ...successItem, success: false, domain: null, status: 'error' };

    const result = await pushDomainResult(actor, failureItem);

    assert.deepEqual(result, { saved: true, statusMessage: null });
    assert.deepEqual(actor.pushed, [{ data: failureItem, eventName: undefined }]);
});

test('pushDomainResult reports charge limit before saving successful domain result', async () => {
    const actor = createActorMock({
        isPayPerEvent: true,
        pushResults: [{ chargedCount: 0, eventChargeLimitReached: true }],
    });

    const result = await pushDomainResult(actor, successItem);

    assert.deepEqual(result, {
        saved: false,
        statusMessage: 'Charge limit reached before saving example.com; stopping batch without writing uncharged success results.',
    });
});
