import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const schema = JSON.parse(readFileSync(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

test('input schema supports batch queries before legacy q', () => {
    assert.deepEqual(schema.anyOf, [
        { required: ['queries'] },
        { required: ['q'] },
    ]);
    assert.equal(schema.properties.queries.type, 'array');
    assert.equal(schema.properties.queries.minItems, 1);
    assert.deepEqual(Object.keys(schema.properties).slice(0, 2), ['queries', 'q']);
});
