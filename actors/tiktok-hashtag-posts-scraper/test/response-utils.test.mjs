import assert from 'node:assert/strict';
import test from 'node:test';

import {
    extractChallenges,
    extractPagination,
    extractPosts,
    getChallengeId,
    getChallengeName,
    selectChallengeForHashtag,
} from '../dist/response-utils.js';

test('extractPosts returns top-level array data as-is', () => {
    const posts = [{ aweme_id: '1' }];
    assert.deepEqual(extractPosts(posts), posts);
});

test('extractPosts reads nested posts arrays', () => {
    const posts = [{ aweme_id: '2' }];
    assert.deepEqual(extractPosts({ posts }), posts);
});

test('extractPosts reads challenge videos arrays', () => {
    const videos = [{ aweme_id: '2' }];
    assert.deepEqual(extractPosts({ videos }), videos);
});

test('extractPosts falls back to aweme_list', () => {
    const posts = [{ aweme_id: '3' }];
    assert.deepEqual(extractPosts({ aweme_list: posts }), posts);
});

test('extractPagination prefers explicit cursor when present', () => {
    assert.deepEqual(
        extractPagination({ hasMore: true, cursor: '100', max_cursor: '200', min_cursor: '300' }),
        { hasNextPage: true, nextCursor: '100' },
    );
});

test('extractPagination falls back through max_cursor and min_cursor', () => {
    assert.deepEqual(
        extractPagination({ has_more: true, max_cursor: '200' }),
        { hasNextPage: true, nextCursor: '200' },
    );
    assert.deepEqual(
        extractPagination({ has_more: false, min_cursor: '300' }),
        { hasNextPage: false, nextCursor: '300' },
    );
});

test('extractPagination handles array or null response data', () => {
    assert.deepEqual(extractPagination([{ aweme_id: '1' }]), { hasNextPage: false, nextCursor: null });
    assert.deepEqual(extractPagination(null), { hasNextPage: false, nextCursor: null });
});

test('extractChallenges reads known search response shapes', () => {
    const challenges = [{ id: '33380', cha_name: 'Cosplay' }];
    assert.deepEqual(extractChallenges(challenges), challenges);
    assert.deepEqual(extractChallenges({ challenges }), challenges);
    assert.deepEqual(extractChallenges({ challenge_list: challenges }), challenges);
});

test('selectChallengeForHashtag prefers exact case-insensitive match', () => {
    const selected = selectChallengeForHashtag([
        { id: '1', cha_name: 'cosplaygirl' },
        { id: '33380', cha_name: 'Cosplay' },
    ], 'cosplay');

    assert.deepEqual(selected, {
        challenge: { id: '33380', cha_name: 'Cosplay' },
        isExactMatch: true,
    });
});

test('selectChallengeForHashtag marks first-result fallback as approximate', () => {
    const selected = selectChallengeForHashtag([
        { id: '1637342470396934', cha_name: 'fypシ' },
    ], 'fyp');

    assert.deepEqual(selected, {
        challenge: { id: '1637342470396934', cha_name: 'fypシ' },
        isExactMatch: false,
    });
});

test('selectChallengeForHashtag returns null when search has no challenges', () => {
    assert.equal(selectChallengeForHashtag([], 'missing'), null);
});

test('getChallengeId and getChallengeName support alternate field names', () => {
    assert.equal(getChallengeId({ challenge_id: '33380' }), '33380');
    assert.equal(getChallengeId({ id: '33380' }), '33380');
    assert.equal(getChallengeId({ challenge_id: 33380 }), '33380');
    assert.equal(getChallengeId({ id: 42 }), '42');
    assert.equal(getChallengeName({ challenge_name: 'cosplay' }), 'cosplay');
    assert.equal(getChallengeName({ cha_name: 'Cosplay' }), 'Cosplay');
});

test('getChallengeId rejects missing, blank, and non-finite values', () => {
    assert.equal(getChallengeId({}), null);
    assert.equal(getChallengeId({ challenge_id: '' }), null);
    assert.equal(getChallengeId({ id: '   ' }), null);
    assert.equal(getChallengeId({ id: Infinity }), null);
    assert.equal(getChallengeId({ challenge_id: NaN }), null);
});
