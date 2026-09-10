import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { Actor } from 'apify';
const source = process.env.TEST_SOURCE === 'src' ? '../src' : '../dist';
const extension = process.env.TEST_SOURCE === 'src' ? 'ts' : 'js';
const { getDiscoveryQuery, planDiscoveredListings } = await import(`${source}/request-params.${extension}`);

const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url)));
const prefill = Object.fromEntries(Object.entries(schema.properties)
    .filter(([, property]) => 'prefill' in property).map(([name, property]) => [name, property.prefill]));

test('discovery validates input, preserves explicit IDs, and bounds distinct candidates', () => {
    assert.deepEqual(prefill, { query: 'fahrrad' });
    assert.equal(getDiscoveryQuery(prefill), 'fahrrad');
    assert.equal(getDiscoveryQuery({ ...prefill, ad_id: '123' }), undefined);
    assert.equal(getDiscoveryQuery({ ...prefill, ad_ids: ['123'] }), undefined);
    assert.throws(() => getDiscoveryQuery({ query: 1 }), /non-empty string/);
    assert.throws(() => getDiscoveryQuery({ ...prefill, ad_ids: [] }), /Provide ad_id/);
    assert.throws(() => planDiscoveredListings({ data: [] }), /Provide ad_id/);
    assert.throws(() => planDiscoveredListings({ data: {} }), /listing array/);
    assert.deepEqual(planDiscoveredListings({ data: [null, {}, { id: 'bad' }, ...['1', '1', '2', '3', '4'].map(id => ({ id }))] }).listings,
        ['1', '2', '3'].map((adId, index) => ({ adId, index })));
});

test('prefilled main saves one charged detail after a removed candidate; all failures fail the run', async (t) => {
    let allRemoved = false;
    let requests = [];
    let writes = [];
    let output;
    let failed;
    let exited = false;
    t.mock.method(Actor, 'init', async () => {});
    t.mock.method(Actor, 'getInput', async () => prefill);
    t.mock.method(Actor, 'getChargingManager', () => ({
        getPricingInfo: () => ({ isPayPerEvent: true }),
        calculateMaxEventChargeCountWithinLimit: () => 10,
    }));
    t.mock.method(Actor, 'pushData', async (data, event) => {
        writes.push({ data, event });
        return { chargedCount: 1 };
    });
    t.mock.method(Actor, 'openKeyValueStore', async () => ({ setValue: async (_, value) => { output = value; } }));
    t.mock.method(Actor, 'fail', async message => { failed = message; });
    t.mock.method(Actor, 'exit', async () => { exited = true; });
    t.mock.method(globalThis, 'fetch', async url => {
        const request = new URL(url);
        requests.push(request);
        if (request.pathname.endsWith('/search')) {
            assert.equal(request.searchParams.get('query'), 'fahrrad');
            return Response.json({ data: ['1', '2', '3', '4'].map(id => ({ id })) });
        }
        const id = request.searchParams.get('ad_id');
        if (allRemoved || id === '1') return Response.json({ message: 'The requested ad does not exist or has been removed.' }, { status: 404 });
        return Response.json({ data: { id, title: 'Current bicycle', description: 'Full listing detail' } });
    });
    const previousKey = process.env.SCRAPPA_API_KEY;
    process.env.SCRAPPA_API_KEY = 'test-only';
    t.after(() => {
        if (previousKey === undefined) delete process.env.SCRAPPA_API_KEY;
        else process.env.SCRAPPA_API_KEY = previousKey;
    });
    await import(`${source}/main.${extension}?success`);
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(failed, undefined);
    assert.equal(exited, true);
    assert.equal(requests.length, 3);
    assert.equal(writes.length, 1);
    assert.equal(writes[0].event, 'listing-detail-result');
    assert.equal(writes[0].data.id, '2');
    assert.equal(output.listings_saved, 1);
    assert.equal(output.listings_failed, 1);

    allRemoved = true;
    requests = []; writes = []; exited = false;
    await import(`${source}/main.${extension}?failure`);
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(exited, false);
    assert.match(failed, /All 3 requested/);
    assert.equal(requests.length, 4);
    assert.equal(writes.length, 0);
    assert.equal(output.listings_failed, 3);
});
