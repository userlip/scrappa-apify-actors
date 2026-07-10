import assert from 'node:assert/strict';
import test from 'node:test';

const modulePath = process.env.TEST_SOURCE === 'src' ? '../src/response-utils.ts' : '../dist/response-utils.js';
const { buildKleinanzeigenDetailsDatasetItem, selectKleinanzeigenDetail } = await import(modulePath);

test('selects a detail from direct and common wrapped Scrappa envelopes', () => {
    assert.deepEqual(selectKleinanzeigenDetail({ data: { id: '1' } }), { id: '1' });
    assert.deepEqual(selectKleinanzeigenDetail({ data: { listing: { id: '2' } } }), { id: '2' });
    assert.deepEqual(selectKleinanzeigenDetail({ result: { id: '3' } }), { id: '3' });
    assert.equal(selectKleinanzeigenDetail({}), null);
});

test('builds required detail fields without mutating the payload', () => {
    const detail = { title: 'Bike', price: '120 €', seller: { name: 'A' }, images: ['image'], categories: ['bikes'] };
    const item = buildKleinanzeigenDetailsDatasetItem(detail, '3451021120', 0);
    assert.deepEqual(detail, { title: 'Bike', price: '120 €', seller: { name: 'A' }, images: ['image'], categories: ['bikes'] });
    assert.deepEqual(item, { ...detail, id: '3451021120', title: 'Bike', price: '120 €', price_numeric: null, description: null, location: null, images: ['image'], seller: { name: 'A' }, attributes: null, shipping: null, posted_at: null, categories: ['bikes'], request_ad_id: '3451021120', request_index: 0 });
});
