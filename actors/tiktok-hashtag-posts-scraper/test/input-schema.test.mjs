import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
const hashtagPattern = new RegExp(schema.properties.hashtag.pattern);

test('hashtag schema accepts challenge IDs up to 100 digits', () => {
    assert.equal(hashtagPattern.test('1'.repeat(100)), true);
});

test('hashtag schema rejects pure numeric values over 100 digits', () => {
    assert.equal(hashtagPattern.test('1'.repeat(101)), false);
    assert.equal(hashtagPattern.test('1'.repeat(150)), false);
});

test('hashtag schema still accepts long non-numeric hashtag names', () => {
    assert.equal(hashtagPattern.test(`${'1'.repeat(101)}a`), true);
    assert.equal(hashtagPattern.test(`#${'1'.repeat(150)}`), true);
});
