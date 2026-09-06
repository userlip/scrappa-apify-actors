import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { buildGoogleVideosParamList } from '../dist/request-params.js';

const schema = JSON.parse(readFileSync(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

test('QA prefill makes one request while retaining batch input', () => {
    const input = Object.fromEntries(Object.entries(schema.properties)
        .filter(([, property]) => property.prefill !== undefined || property.default !== undefined)
        .map(([name, property]) => [name, property.prefill ?? property.default]));
    assert.equal(buildGoogleVideosParamList(input).length, 1);
    assert.equal(schema.properties.q.prefill, undefined);
    assert.equal(buildGoogleVideosParamList({ queries: ['coffee', 'espresso'] }).length, 2);
});

test('does not default page so start offset pagination remains usable', () => {
    assert.equal(schema.required, undefined);
    assert.equal(schema.anyOf, undefined);
    assert.equal(schema.properties.queries.type, 'array');
    assert.equal(schema.properties.queries.minItems, 1);
    assert.deepEqual(Object.keys(schema.properties).slice(0, 2), ['queries', 'q']);
    assert.equal(schema.properties.page.default, undefined);
    assert.equal(schema.properties.start.default, undefined);
});
