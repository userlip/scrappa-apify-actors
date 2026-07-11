import assert from 'node:assert/strict';
import test from 'node:test';
import { parseInput } from '../dist/input.js';

test('creates one bounded request per unique challenge ID', () => {
    assert.deepEqual(parseInput({ challenge_ids: ['1', '2', '1'], region: 'us', results_per_challenge: 20, page_size: 50 }), [
        { challengeId: '1', region: 'US', initialCursor: undefined, resultLimit: 20, pageSize: 20 },
        { challengeId: '2', region: 'US', initialCursor: undefined, resultLimit: 20, pageSize: 20 },
    ]);
});

test('supports legacy single challenge_id and numeric cursor', () => {
    assert.deepEqual(parseInput({ challenge_id: '1622962893630470', cursor: 10 }), [
        { challengeId: '1622962893630470', region: undefined, initialCursor: '10', resultLimit: 100, pageSize: 10 },
    ]);
});

test('enforces challenge and total-result caps', () => {
    assert.throws(() => parseInput({ challenge_ids: Array.from({ length: 21 }, (_, i) => String(i + 1)) }), /maximum of 20/);
    assert.throws(() => parseInput({ challenge_ids: ['1', '2', '3', '4', '5'], results_per_challenge: 500 }), /cannot exceed 2000/);
});

test('rejects missing IDs, malformed region, and invalid limits', () => {
    assert.throws(() => parseInput({ challenge_ids: ['abc'] }), /numeric TikTok challenge ID/);
    assert.throws(() => parseInput({ challenge_id: '1', region: 'USA' }), /two-letter/);
    assert.throws(() => parseInput({ challenge_id: '1', region: 123 }), /two-letter/);
    assert.throws(() => parseInput({ challenge_id: '1', page_size: 0 }), /page_size/);
});
