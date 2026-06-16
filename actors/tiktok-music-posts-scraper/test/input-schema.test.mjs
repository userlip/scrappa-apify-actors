import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
const musicIdPattern = new RegExp(schema.properties.musicIds.items.pattern);

test('musicIds schema accepts music IDs up to 100 digits', () => {
    assert.equal(musicIdPattern.test('1'.repeat(100)), true);
});

test('musicIds schema rejects pure numeric values over 100 digits', () => {
    assert.equal(musicIdPattern.test('1'.repeat(101)), false);
    assert.equal(musicIdPattern.test('1'.repeat(150)), false);
});

test('musicIds schema rejects non-numeric music IDs', () => {
    assert.equal(musicIdPattern.test('track-name'), false);
});

test('cursor schema uses a single Apify-compatible type', () => {
    assert.equal(schema.properties.cursor.type, 'string');
    assert.equal(schema.properties.cursor.editor, 'textfield');
});

test('legacy music_id is not prefilled alongside musicIds', () => {
    assert.equal(Object.hasOwn(schema.properties.music_id, 'prefill'), false);
});
