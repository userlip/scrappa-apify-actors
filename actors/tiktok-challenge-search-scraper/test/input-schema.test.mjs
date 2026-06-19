import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

test('keywords schema uses batch string list input', () => {
    assert.equal(schema.properties.keywords.type, 'array');
    assert.equal(schema.properties.keywords.editor, 'stringList');
    assert.equal(schema.properties.keywords.items.type, 'string');
});

test('legacy keyword is optional so API callers can use single-keyword input', () => {
    assert.equal(schema.properties.keyword.type, 'string');
    assert.equal(Object.hasOwn(schema, 'required'), false);
});

test('count schema keeps the Scrappa challenge search limit bounded', () => {
    assert.equal(schema.properties.count.minimum, 1);
    assert.equal(schema.properties.count.maximum, 50);
    assert.equal(schema.properties.count.default, 10);
});
