import assert from 'node:assert/strict';
import test from 'node:test';
const modulePath = process.env.TEST_SOURCE === 'src' ? '../src/request-params.ts' : '../dist/request-params.js';
const { normalizeLocations } = await import(modulePath);

test('normalizes and deduplicates an array of locations', () => {
    assert.deepEqual(normalizeLocations({ locations: [' Berlin ', 'Munich', 'berlin', ''] }), [
        { location: 'Berlin', index: 0 },
        { location: 'Munich', index: 1 },
    ]);
});

test('accepts comma-separated locations and singular compatibility input', () => {
    assert.deepEqual(normalizeLocations({ locations: 'Berlin, Munich' }).map((item) => item.location), ['Berlin', 'Munich']);
    assert.deepEqual(normalizeLocations({ location: 'Hamburg' }), [{ location: 'Hamburg', index: 0 }]);
});

test('rejects missing and invalid locations', () => {
    assert.throws(() => normalizeLocations({}), /at least one non-empty location/);
    assert.throws(() => normalizeLocations({ locations: 42 }), /array of strings or a comma-separated string/);
});
