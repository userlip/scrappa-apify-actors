import assert from 'node:assert/strict';
import test from 'node:test';
import { buildTikTokChallengeDetailsRequests, normalizeChallengeId, normalizeChallengeName } from '../dist/request-params.js';

test('normalizes, deduplicates, and mixes names and IDs', () => {
    assert.deepEqual(buildTikTokChallengeDetailsRequests({
        challenge_names: [' #BookTok ', 'booktok', 'fitness'],
        challenge_ids: ['1622962893630470', '1622962893630470'],
        challenge_name: 'Fitness', challenge_id: 42,
    }), [
        { type: 'challenge_name', value: 'BookTok', params: { challenge_name: 'BookTok' } },
        { type: 'challenge_name', value: 'fitness', params: { challenge_name: 'fitness' } },
        { type: 'challenge_id', value: '1622962893630470', params: { challenge_id: '1622962893630470' } },
        { type: 'challenge_id', value: '42', params: { challenge_id: '42' } },
    ]);
});

test('valid batch values never suppress compatible single values', () => {
    const requests = buildTikTokChallengeDetailsRequests({ challenge_names: ['booktok'], challenge_id: '1' });
    assert.equal(requests.length, 2);
    assert.deepEqual(requests[0].params, { challenge_name: 'booktok' });
    assert.deepEqual(requests[1].params, { challenge_id: '1' });
});

test('omits malformed list entries and rejects no valid values', () => {
    const warnings = [];
    assert.deepEqual(buildTikTokChallengeDetailsRequests({ challenge_names: ['booktok', 'bad/name', 42] }, (message) => warnings.push(message)), [
        { type: 'challenge_name', value: 'booktok', params: { challenge_name: 'booktok' } },
    ]);
    assert.equal(warnings.length, 2);
    assert.match(warnings[0], /omitted/);
    assert.throws(() => buildTikTokChallengeDetailsRequests({ challenge_ids: ['not-an-id'] }, () => {}), /At least one valid/);
});

test('enforces normalization constraints and combined maximum', () => {
    assert.equal(normalizeChallengeName('#booktok'), 'booktok');
    assert.equal(normalizeChallengeId(123), '123');
    assert.throws(() => normalizeChallengeId(Number.MAX_SAFE_INTEGER + 1), /IDs/);
    assert.throws(() => buildTikTokChallengeDetailsRequests({ challenge_names: Array.from({ length: 101 }, (_, index) => `tag${index}`) }), /maximum of 100/);
});
