import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

const schemaUrl = new URL('../.actor/input_schema.json', import.meta.url);

test('schema defaults always provide a complete QA smoke-test route', async () => {
    const schema = JSON.parse(await readFile(schemaUrl, 'utf8'));
    const { origin, destination } = schema.properties;

    assert.equal(typeof origin.default, 'string');
    assert.ok(origin.default.trim());
    assert.equal(origin.prefill, origin.default);

    assert.equal(typeof destination.default, 'string');
    assert.ok(destination.default.trim());
    assert.equal(destination.prefill, destination.default);
});
