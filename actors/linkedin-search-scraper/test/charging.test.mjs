import assert from 'node:assert/strict';
import test from 'node:test';

import {
    getLinkedInSearchChargeLimitStatus,
    getRemainingLinkedInSearchResultCharges,
    LINKEDIN_SEARCH_RESULT_CHARGE_EVENT,
    pushLinkedInSearchResult,
} from '../dist/charging.js';

function createActor({ isPayPerEvent = true, remainingCharges = 1, pushResults = [] } = {}) {
    const pushed = [];

    return {
        pushed,
        getChargingManager() {
            return {
                getPricingInfo() {
                    return { isPayPerEvent };
                },
                calculateMaxEventChargeCountWithinLimit(eventName) {
                    assert.equal(eventName, LINKEDIN_SEARCH_RESULT_CHARGE_EVENT);
                    return remainingCharges;
                },
            };
        },
        async pushData(item, eventName) {
            pushed.push({ item, eventName });
            return pushResults.shift() ?? {
                chargedCount: eventName ? 1 : 0,
                eventChargeLimitReached: false,
            };
        },
    };
}

test('reports no remaining charge limit outside pay-per-event', () => {
    const actor = createActor({ isPayPerEvent: false });

    assert.equal(getRemainingLinkedInSearchResultCharges(actor), null);
});

test('returns remaining LinkedIn search result charge count for pay-per-event runs', () => {
    const actor = createActor({ remainingCharges: 3 });

    assert.equal(getRemainingLinkedInSearchResultCharges(actor), 3);
});

test('reports charge limit status when pay-per-event result budget is exhausted', () => {
    const actor = createActor({ remainingCharges: 0 });

    assert.equal(
        getLinkedInSearchChargeLimitStatus(actor, 1, 3),
        'Charge limit reached before saving the next LinkedIn search result; 1 of 3 result(s) were saved.',
    );
});

test('pushLinkedInSearchResult charges successful results in pay-per-event runs', async () => {
    const actor = createActor();
    const result = { title: 'Founder', link: 'https://www.linkedin.com/in/founder' };

    assert.deepEqual(await pushLinkedInSearchResult(actor, result), { saved: true, statusMessage: null });
    assert.deepEqual(actor.pushed, [{ item: result, eventName: LINKEDIN_SEARCH_RESULT_CHARGE_EVENT }]);
});

test('pushLinkedInSearchResult saves without event outside pay-per-event runs', async () => {
    const actor = createActor({ isPayPerEvent: false });
    const result = { title: 'Founder', link: 'https://www.linkedin.com/in/founder' };

    assert.deepEqual(await pushLinkedInSearchResult(actor, result), { saved: true, statusMessage: null });
    assert.deepEqual(actor.pushed, [{ item: result, eventName: undefined }]);
});

test('pushLinkedInSearchResult reports unsaved result when event charge is exhausted', async () => {
    const actor = createActor({
        pushResults: [{ chargedCount: 0, eventChargeLimitReached: true }],
    });
    const originalWarn = console.warn;
    console.warn = () => {};

    try {
        const result = { title: 'Founder', link: 'https://www.linkedin.com/in/founder' };
        assert.deepEqual(await pushLinkedInSearchResult(actor, result), {
            saved: false,
            statusMessage: 'Charge limit reached before saving the LinkedIn search result; stopping without writing uncharged results.',
        });
    } finally {
        console.warn = originalWarn;
    }
});
