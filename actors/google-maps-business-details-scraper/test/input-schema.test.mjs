import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const inputSchema = JSON.parse(
    await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'),
);

test('input schema supports batch business_ids before legacy business_id', () => {
    assert.equal(inputSchema.properties.business_ids.type, 'array');
    assert.equal(inputSchema.properties.business_ids.items.type, 'string');
    assert.equal(inputSchema.properties.business_id.type, 'string');
    assert.deepEqual(Object.keys(inputSchema.properties).slice(0, 2), ['business_ids', 'business_id']);
    assert.equal(inputSchema.required, undefined);
});
