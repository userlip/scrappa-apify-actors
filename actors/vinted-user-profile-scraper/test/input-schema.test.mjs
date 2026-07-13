import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

test('declares array and CSV string batch inputs for Apify validation', () => {
    const userIds = schema.properties.user_ids;
    const acceptsDeclaredType = (value) => userIds.type.includes(Array.isArray(value) ? 'array' : typeof value);

    assert.deepEqual([...userIds.type].sort(), ['array', 'string']);
    assert.equal(userIds.editor, 'json');
    assert.equal(acceptsDeclaredType(['123', '456']), true);
    assert.equal(acceptsDeclaredType('123,456'), true);
    assert.match(userIds.description, /numeric IDs/);
    assert.match(userIds.description, /maximum of 100 unique IDs/);
});
