import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';

const inputSchema = JSON.parse(fs.readFileSync(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

test('input schema supports legacy url and batch urls fields', () => {
    assert.equal(inputSchema.properties.url.type, 'string');
    assert.equal(inputSchema.properties.urls.type, 'array');
    assert.equal(inputSchema.properties.urls.items.type, 'string');
    assert.deepEqual(Object.keys(inputSchema.properties).slice(0, 2), ['urls', 'url']);
    assert.equal(inputSchema.required, undefined);
});
