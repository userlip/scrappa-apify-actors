import assert from 'node:assert/strict';
import test from 'node:test';

const module = process.env.TEST_SOURCE === 'src'
    ? '../src/listing-processing.ts'
    : '../dist/listing-processing.js';
const {
    LISTING_DETAIL_RESULT_CHARGE_EVENT,
    buildListingDetailsOutput,
    processKleinanzeigenListingDetails,
} = await import(module);

function actor({ isPayPerEvent, capacity = 10, chargeResults = [] }) {
    const writes = [];
    return {
        writes,
        getChargingManager() {
            return {
                getPricingInfo: () => ({ isPayPerEvent }),
                calculateMaxEventChargeCountWithinLimit: (event) => {
                    assert.equal(event, LISTING_DETAIL_RESULT_CHARGE_EVENT);
                    return capacity;
                },
            };
        },
        async pushData(data, eventName) {
            writes.push({ data, eventName });
            return chargeResults.shift() ?? { chargedCount: 1, eventChargeLimitReached: false };
        },
    };
}

const twoListings = [{ adId: '1', index: 0 }, { adId: '2', index: 1 }];
const responseFor = (id) => ({ data: { id, title: `Listing ${id}` } });

test('non-pay-per-event runs save every successful listing without a charge event', async () => {
    const fakeActor = actor({ isPayPerEvent: false, capacity: 0 });
    const result = await processKleinanzeigenListingDetails(fakeActor, twoListings, async (id) => responseFor(id));

    assert.equal(result.savedCount, 2);
    assert.equal(result.statusMessage, null);
    assert.equal(result.failures.length, 0);
    assert.deepEqual(buildListingDetailsOutput(twoListings.length, result), {
        listings_requested: 2,
        listings_completed: 2,
        listings_saved: 2,
        listings_failed: 0,
        status_message: null,
        failures: [],
    });
    assert.deepEqual(fakeActor.writes.map(({ eventName }) => eventName), [undefined, undefined]);
    assert.deepEqual(fakeActor.writes.map(({ data }) => data.request_ad_id), ['1', '2']);
});

test('pay-per-event runs stop before fetching when no charge capacity remains', async () => {
    const fakeActor = actor({ isPayPerEvent: true, capacity: 0 });
    let fetches = 0;
    const result = await processKleinanzeigenListingDetails(fakeActor, twoListings, async (id) => {
        fetches++;
        return responseFor(id);
    });

    assert.equal(fetches, 0);
    assert.equal(result.savedCount, 0);
    assert.match(result.statusMessage, /before fetching Kleinanzeigen listing 1/);
});

test('pay-per-event runs stop after a short charge result', async () => {
    const fakeActor = actor({ isPayPerEvent: true, chargeResults: [{ chargedCount: 0, eventChargeLimitReached: true }] });
    const result = await processKleinanzeigenListingDetails(fakeActor, twoListings, async (id) => responseFor(id));

    assert.equal(result.savedCount, 0);
    assert.match(result.statusMessage, /after saving 0/);
    assert.equal(fakeActor.writes.length, 1);
    assert.equal(fakeActor.writes[0].eventName, LISTING_DETAIL_RESULT_CHARGE_EVENT);
});

test('continues after a per-listing request failure', async () => {
    const fakeActor = actor({ isPayPerEvent: true });
    const result = await processKleinanzeigenListingDetails(fakeActor, twoListings, async (id) => {
        if (id === '1') throw new Error('request failed');
        return responseFor(id);
    });

    assert.equal(result.savedCount, 1);
    assert.equal(result.failures.length, 1);
    assert.equal(result.failures[0].ad_id, '1');
    assert.equal(fakeActor.writes.length, 1);
    assert.equal(fakeActor.writes[0].data.request_ad_id, '2');
});
