import assert from 'node:assert/strict';
import test from 'node:test';

const chargingUtilsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/charging-utils.ts'
    : '../dist/charging-utils.js';
const {
    getPinterestChargedSaveResult,
} = await import(chargingUtilsModule);

test('normalizes charged save counts and partial charge limits', () => {
    assert.deepEqual(
        getPinterestChargedSaveResult({ chargedCount: 3, eventChargeLimitReached: false }, 3),
        { savedCount: 3, chargeLimitReached: false },
    );
    assert.deepEqual(
        getPinterestChargedSaveResult({ chargedCount: 2, eventChargeLimitReached: false }, 3),
        { savedCount: 2, chargeLimitReached: true },
    );
    assert.deepEqual(
        getPinterestChargedSaveResult({ chargedCount: 5, eventChargeLimitReached: true }, 3),
        { savedCount: 3, chargeLimitReached: true },
    );
});
