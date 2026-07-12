import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
const indices = schema.properties.indices;

function accepts(value) {
    if (typeof value === 'string') return indices.type.includes('string');
    return Array.isArray(value)
        && indices.type.includes('array');
}

test('published contract accepts CSV and JSON-array batches', () => {
    assert.equal(indices.editor, 'json');
    assert.equal(accepts('.INX,.DJI'), true);
    assert.equal(accepts(['.INX', '.DJI']), true);
});
