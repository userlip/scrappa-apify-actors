import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const modulePath = process.env.TEST_SOURCE === 'src' ? '../src/request-params.ts' : '../dist/request-params.js';
const { buildKleinanzeigenDetailsPlan, describeKleinanzeigenDetailsRequest } = await import(modulePath);

test('plans single, batch, and combined IDs in stable deduplicated order', () => {
    assert.deepEqual(buildKleinanzeigenDetailsPlan({ ad_id: ' 3451021120 ' }), { listings: [{ adId: '3451021120', index: 0 }] });
    const plan = buildKleinanzeigenDetailsPlan({ ad_id: '1', ad_ids: ['2', '1', 3] });
    assert.deepEqual(plan, { listings: [{ adId: '1', index: 0 }, { adId: '2', index: 1 }, { adId: '3', index: 2 }] });
    assert.equal(describeKleinanzeigenDetailsRequest(plan), '3 Kleinanzeigen listings');
});

test('validates supplied values and the unique-ID limit', () => {
    assert.throws(() => buildKleinanzeigenDetailsPlan({}), /Provide ad_id/);
    assert.throws(() => buildKleinanzeigenDetailsPlan({ ad_id: ' ' }), /cannot be blank/);
    assert.throws(() => buildKleinanzeigenDetailsPlan({ ad_id: '12.5' }), /only digits/);
    assert.throws(() => buildKleinanzeigenDetailsPlan({ ad_id: 1.5 }), /safe integer/);
    assert.throws(() => buildKleinanzeigenDetailsPlan({ ad_ids: '1' }), /must be an array/);
    assert.equal(buildKleinanzeigenDetailsPlan({ ad_ids: Array.from({ length: 100 }, (_, i) => String(i)) }).listings.length, 100);
    assert.throws(() => buildKleinanzeigenDetailsPlan({ ad_id: '100', ad_ids: Array.from({ length: 100 }, (_, i) => String(i)) }), /maximum of 100/);
});

test('input schema documents the batched ID contract', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
    assert.equal(schema.required, undefined);
    assert.equal(schema.properties.ad_id.type, 'string');
    assert.equal(schema.properties.ad_ids.maxItems, 100);
    assert.equal(schema.properties.ad_ids.items.type, 'string');
});
