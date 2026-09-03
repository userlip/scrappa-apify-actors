import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const schema = JSON.parse(readFileSync(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

test('input schema supports batch queries before legacy q', () => {
    assert.equal(schema.required, undefined);
    assert.equal(schema.anyOf, undefined);
    assert.equal(schema.properties.queries.type, 'array');
    assert.equal(schema.properties.queries.minItems, 1);
    assert.deepEqual(Object.keys(schema.properties).slice(0, 2), ['queries', 'q']);
});

test('prefilled QA input uses the monitored Google Images request', () => {
    assert.deepEqual(schema.properties.queries.prefill, ['coffee']);
    assert.equal(schema.properties.q.prefill, undefined);
    assert.equal(schema.properties.page.default, 1);
    assert.equal(schema.properties.hl.default, 'en');
    assert.equal(schema.properties.gl.default, 'us');
    assert.equal(schema.properties.safe.default, 'active');
});
