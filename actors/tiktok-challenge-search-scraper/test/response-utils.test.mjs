import assert from 'node:assert/strict';
import test from 'node:test';

import {
    extractChallenges,
    getChallengeId,
    getChallengeName,
    normalizeChallengeResult,
} from '../dist/response-utils.js';

test('extractChallenges returns top-level array data as-is', () => {
    const challenges = [{ challenge_id: '1' }];
    assert.deepEqual(extractChallenges(challenges), challenges);
});

test('extractChallenges reads common nested challenge arrays', () => {
    const challenges = [{ challenge_id: '2' }];
    const challengeList = [{ challenge_id: '3' }];
    const camelChallengeList = [{ challenge_id: '4' }];

    assert.deepEqual(extractChallenges({ challenges }), challenges);
    assert.deepEqual(extractChallenges({ challenge_list: challengeList }), challengeList);
    assert.deepEqual(extractChallenges({ challengeList: camelChallengeList }), camelChallengeList);
});

test('extractChallenges falls back through generic item arrays', () => {
    const items = [{ challenge_id: '5' }];
    const results = [{ challenge_id: '6' }];

    assert.deepEqual(extractChallenges({ items }), items);
    assert.deepEqual(extractChallenges({ results }), results);
});

test('extractChallenges returns an empty array for missing challenge arrays', () => {
    assert.deepEqual(extractChallenges(null), []);
    assert.deepEqual(extractChallenges({}), []);
});

test('getChallengeId supports known id fields and numeric ids', () => {
    assert.equal(getChallengeId({ challenge_id: ' 123 ' }), '123');
    assert.equal(getChallengeId({ id: 456 }), '456');
    assert.equal(getChallengeId({ cid: '789' }), '789');
    assert.equal(getChallengeId({ challenge_id: ' ' }), null);
});

test('getChallengeName supports known name fields', () => {
    assert.equal(getChallengeName({ challenge_name: ' cosplay ' }), 'cosplay');
    assert.equal(getChallengeName({ cha_name: 'fitness' }), 'fitness');
    assert.equal(getChallengeName({ name: 'skincare' }), 'skincare');
    assert.equal(getChallengeName({ title: 'makeup' }), 'makeup');
    assert.equal(getChallengeName({ title: ' ' }), null);
});

test('normalizeChallengeResult preserves raw fields and adds normalized request metadata', () => {
    assert.deepEqual(
        normalizeChallengeResult(
            {
                id: '123',
                cha_name: 'cosplay',
                desc: 'Costume videos',
                stats: {
                    view_count: 100,
                    video_count: 20,
                    user_count: 5,
                },
                extra: 'raw',
            },
            { keyword: 'cosplay', count: 10 },
        ),
        {
            id: '123',
            cha_name: 'cosplay',
            desc: 'Costume videos',
            stats: {
                view_count: 100,
                video_count: 20,
                user_count: 5,
            },
            extra: 'raw',
            challenge_id: '123',
            challenge_name: 'cosplay',
            description: 'Costume videos',
            view_count: 100,
            video_count: 20,
            user_count: 5,
            request_keyword: 'cosplay',
            request_count: 10,
        },
    );
});
