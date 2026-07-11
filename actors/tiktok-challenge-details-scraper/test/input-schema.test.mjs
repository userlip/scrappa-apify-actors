import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

test('schema makes batch names and IDs the preferred string-list inputs', () => {
    assert.equal(schema.properties.challenge_names.editor, 'stringList');
    assert.equal(schema.properties.challenge_names.items.maxLength, 255);
    assert.equal(schema.properties.challenge_ids.editor, 'stringList');
    assert.equal(schema.properties.challenge_ids.items.maxLength, 100);
    assert.match(schema.properties.challenge_names.description, /100/);
});

test('schema keeps optional single-value compatibility fields', () => {
    assert.equal(Object.hasOwn(schema, 'required'), false);
    assert.equal(schema.properties.challenge_name.maxLength, 255);
    assert.equal(schema.properties.challenge_id.maxLength, 100);
});

test('schema prevents the broken union-type textfield pattern that fails Apify deployment', () => {
    const field = schema.properties.challenge_id;

    // Apify accepts `textfield` only for a single string schema type. Numeric
    // legacy input remains supported by normalizeChallengeId at runtime.
    assert.equal(field.editor, 'textfield');
    assert.equal(field.type, 'string');
    assert.equal(Array.isArray(field.type), false);
    assert.notDeepEqual(field.type, ['string', 'integer']);
});
