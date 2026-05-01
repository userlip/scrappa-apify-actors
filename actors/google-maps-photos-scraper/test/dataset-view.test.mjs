import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

test('prioritizes populated Google Maps photo fields in the dataset table', async () => {
    const actorConfig = JSON.parse(
        await readFile(new URL('../.actor/actor.json', import.meta.url), 'utf8'),
    );

    const overview = actorConfig.storages.dataset.views.overview;

    assert.deepEqual(overview.transformation.fields, [
        'photo_url_large',
        'width',
        'height',
        'contributor_name',
        'posted_at',
        'photo_id',
    ]);

    assert.equal(overview.display.properties.photo_url_large.label, 'Photo');
    assert.equal(overview.display.properties.photo_url_large.format, 'link');
    assert.equal(overview.display.properties.width.format, 'number');
    assert.equal(overview.display.properties.height.format, 'number');
    assert.equal(overview.display.properties.contributor_name.label, 'Contributor');
    assert.equal(overview.display.properties.posted_at.label, 'Posted');
    assert.equal(overview.display.properties.photo_id.label, 'ID');
});
