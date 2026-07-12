import assert from 'node:assert/strict'; import { readFile } from 'node:fs/promises'; import test from 'node:test';
const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
function accepts(value) { const type = schema.properties.indices.type; return (typeof value === 'string' && type.includes('string')) || (Array.isArray(value) && type.includes('array')); }
test('published contract accepts both batch forms', () => { assert.equal(schema.properties.indices.editor, 'json'); assert.equal(accepts('.INX,.DJI'), true); assert.equal(accepts(['.INX', '.DJI']), true); });
