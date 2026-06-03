import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const inputSchema = JSON.parse(
    await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'),
);

test('input schema supports url and urls fields', () => {
    assert.equal(inputSchema.properties.url.type, 'string');
    assert.equal(inputSchema.properties.urls.type, 'array');
    assert.equal(inputSchema.properties.urls.items.type, 'string');
    assert.deepEqual(Object.keys(inputSchema.properties).slice(0, 2), ['urls', 'url']);
});

test('input schema rejects maximum_cache_age values below 1 before Scrappa receives them', () => {
    assert.equal(inputSchema.properties.maximum_cache_age.type, 'integer');
    assert.equal(inputSchema.properties.maximum_cache_age.minimum, 1);
});
