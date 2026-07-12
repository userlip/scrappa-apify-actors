import assert from 'node:assert/strict';
import test from 'node:test';

const modulePath = process.env.TEST_SOURCE === 'src' ? '../src/locations.ts' : '../dist/locations.js';
const { buildUniqueLocationItems, getLocations } = await import(modulePath);

test('maps top-level and wrapped response locations', () => {
    const locations = [{ geocode: '1', name: 'Berlin', type: 'city' }];
    assert.deepEqual(getLocations({ locations }), locations);
    assert.deepEqual(getLocations({ data: { locations } }), locations);
    assert.deepEqual(getLocations({ success: false }), []);
});

test('adds source query and deduplicates geocodes across responses', () => {
    const seen = new Set();
    const berlin = buildUniqueLocationItems({ locations: [
        { geocode: '1', name: 'Berlin', type: 'city' },
        { geocode: '2', name: 'Berlin - Mitte', type: 'district' },
    ] }, 'Berlin', seen);
    const mitte = buildUniqueLocationItems({ locations: [
        { geocode: '2', name: 'Berlin - Mitte', type: 'district' },
        { geocode: '3', name: 'Mitte', type: 'district' },
    ] }, 'Mitte', seen);

    assert.deepEqual(berlin, [
        { geocode: '1', name: 'Berlin', type: 'city', source_query: 'Berlin' },
        { geocode: '2', name: 'Berlin - Mitte', type: 'district', source_query: 'Berlin' },
    ]);
    assert.deepEqual(mitte, [
        { geocode: '3', name: 'Mitte', type: 'district', source_query: 'Mitte' },
    ]);
});

test('drops malformed location rows', () => {
    assert.deepEqual(buildUniqueLocationItems({ locations: [
        null,
        { geocode: '', name: 'Missing code', type: 'city' },
        { geocode: '1', name: null, type: 'city' },
        { geocode: '2', name: 'Valid', type: 'district', extra: true },
    ] }, 'Valid', new Set()), [
        { geocode: '2', name: 'Valid', type: 'district', extra: true, source_query: 'Valid' },
    ]);
});
