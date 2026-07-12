import assert from 'node:assert/strict'; import test from 'node:test';
import { buildGoogleFinanceIndicesParams, normalizeIndices } from '../dist/request-params.js';
test('normalizes CSV and array symbols in first-seen order', () => { assert.deepEqual(normalizeIndices(' .inx, .DJI, .inx '), ['.INX', '.DJI']); assert.deepEqual(normalizeIndices([' .inx ', '.DJI', '.inx']), ['.INX', '.DJI']); });
test('enforces cap and locale', () => { assert.throws(() => normalizeIndices(Array.from({ length: 51 }, (_, i) => `.I${i}`)), /maximum/); assert.deepEqual(buildGoogleFinanceIndicesParams({ indices: ['.INX'], hl: 'EN', gl: 'US' }), { indices: '.INX', hl: 'en', gl: 'us' }); assert.throws(() => buildGoogleFinanceIndicesParams({ gl: 'usa' }), /two-letter/); });
