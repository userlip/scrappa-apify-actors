import assert from 'node:assert/strict';
import test from 'node:test';
import { extractChallengeDetail, normalizeChallengeDetail } from '../dist/response-utils.js';

test('maps booktok-style detail data while retaining raw fields', () => {
    const result = normalizeChallengeDetail({
        id: '1622962893630470', cha_name: 'BookTok', desc: 'Books', user_count: 5,
        view_count: 10, video_count: 3, cover: 'https://example.test/cover.jpg', is_commerce: false, raw: 'kept',
    }, { type: 'challenge_name', value: 'booktok' }, '2026-07-11T00:00:00.000Z');
    assert.equal(result.challenge_id, '1622962893630470');
    assert.equal(result.challenge_name, 'BookTok');
    assert.equal(result.user_count, 5);
    assert.equal(result.raw, 'kept');
    assert.equal(result.request_challenge_name, 'booktok');
    assert.equal(result.retrieved_at, '2026-07-11T00:00:00.000Z');
});

test('uses null rather than a false zero for absent metrics', () => {
    const result = normalizeChallengeDetail({ challenge_id: '1', challenge_name: 'one' }, { type: 'challenge_id', value: '1' });
    assert.equal(result.user_count, null); assert.equal(result.view_count, null); assert.equal(result.video_count, null);
    assert.deepEqual(extractChallengeDetail({ challenge: { id: '1' } }), { id: '1' });
    assert.equal(extractChallengeDetail(null), null);
});
