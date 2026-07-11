import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

function validateApifyEditorCompatibility(field) {
    if (field.editor !== 'textfield') return;

    const acceptedEditors = typeof field.type === 'string'
        ? ['textfield', 'textarea', 'select', 'hidden']
        : ['json', 'hidden'];

    assert.ok(
        acceptedEditors.includes(field.editor),
        `Apify does not allow editor ${JSON.stringify(field.editor)} for type ${JSON.stringify(field.type)}`,
    );
}

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

test('schema passes Apify textfield/editor compatibility validation', () => {
    const field = schema.properties.challenge_id;

    validateApifyEditorCompatibility(field);

    assert.throws(
        () => validateApifyEditorCompatibility({ ...field, type: ['string', 'integer'] }),
        /Apify does not allow editor "textfield" for type \["string","integer"\]/,
    );
});
