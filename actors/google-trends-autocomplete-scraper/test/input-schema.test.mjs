import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const schema = JSON.parse(
    await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'),
);

test('input schema allows the q alias to reach runtime validation', () => {
    assert.equal(schema.required, undefined);
    assert.equal(schema.anyOf, undefined);
    assert.equal(schema.properties.query.maxLength, 100);
    assert.equal(schema.properties.q.type, 'string');
    assert.equal(schema.properties.q.maxLength, 100);
    assert.match(schema.properties.q.description, /Alias for query/);
});
