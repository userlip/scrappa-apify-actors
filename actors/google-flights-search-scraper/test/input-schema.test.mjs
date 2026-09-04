import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const schema = JSON.parse(readFileSync(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

test('prefilled QA dates stay in the future relative to each run', () => {
    assert.equal(schema.properties.departure_date.dateType, 'absoluteOrRelative');
    assert.equal(schema.properties.departure_date.prefill, '45 days');
    assert.equal(schema.properties.return_date.dateType, 'absoluteOrRelative');
    assert.equal(schema.properties.return_date.prefill, '52 days');
});
