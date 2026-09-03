import assert from 'node:assert/strict';
import test from 'node:test';

const modulePath = process.env.TEST_SOURCE === 'src' ? '../src/fallback-locations.ts' : '../dist/fallback-locations.js';
const { getFallbackLocations } = await import(modulePath);

test('returns verified Berlin locations case-insensitively and respects the limit', () => {
    assert.deepEqual(getFallbackLocations('berlin', 1), {
        locations: [{
            geocode: '1276003001',
            name: 'Berlin',
            type: 'city',
            is_cached: true,
        }],
    });
});

test('does not fabricate fallback locations for unsupported queries', () => {
    assert.equal(getFallbackLocations('Hamburg', 10), null);
});
