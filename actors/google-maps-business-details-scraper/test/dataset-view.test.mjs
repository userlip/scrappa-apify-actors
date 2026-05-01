import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const __dirname = dirname(fileURLToPath(import.meta.url));
const expectedFields = [
    'name',
    'rating',
    'review_count',
    'full_address',
    'phone_number',
    'website',
    'type',
];

test('dataset overview uses fields emitted by business details results', async () => {
    const actorJson = JSON.parse(
        await readFile(join(__dirname, '../.actor/actor.json'), 'utf8'),
    );

    const view = actorJson.storages.dataset.views.overview;
    const emittedFields = new Set(expectedFields);

    assert.deepEqual(view.transformation.fields, expectedFields);

    for (const field of view.transformation.fields) {
        assert.ok(emittedFields.has(field), `${field} is not emitted by business details results`);
        assert.ok(view.display.properties[field], `${field} is missing display properties`);
    }

    assert.equal(view.display.properties.full_address.label, 'Address');
    assert.equal(view.display.properties.phone_number.label, 'Phone');
    assert.equal(view.display.properties.address, undefined);
    assert.equal(view.display.properties.phone, undefined);
});
