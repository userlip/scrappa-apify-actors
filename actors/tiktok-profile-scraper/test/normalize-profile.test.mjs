import assert from 'node:assert/strict';
import test from 'node:test';

import { normalizeTikTokProfileRecord } from '../src/normalize-profile.ts';

test('normalizes nested Scrappa TikTok profile payload into top-level actor fields', () => {
    const normalized = normalizeTikTokProfileRecord({
        user: {
            id: '107955',
            uniqueId: 'tiktok',
            nickname: 'TikTok',
            avatarThumb: 'thumb.webp',
            avatarMedium: 'medium.webp',
            avatarLarger: 'large.webp',
            signature: 'One TikTok can make a big impact',
            verified: true,
            privateAccount: false,
            region: 'US',
            language: 'en',
        },
        stats: {
            followingCount: 3,
            followerCount: 93950516,
            heartCount: 457861218,
            videoCount: 1509,
            diggCount: 0,
        },
    });

    assert.equal(normalized.user_id, '107955');
    assert.equal(normalized.unique_id, '@tiktok');
    assert.equal(normalized.nickname, 'TikTok');
    assert.equal(normalized.avatar, 'large.webp');
    assert.equal(normalized.signature, 'One TikTok can make a big impact');
    assert.equal(normalized.verified, true);
    assert.equal(normalized.private_account, false);
    assert.equal(normalized.region, 'US');
    assert.equal(normalized.language, 'en');
    assert.equal(normalized.following_count, 3);
    assert.equal(normalized.follower_count, 93950516);
    assert.equal(normalized.heart_count, 457861218);
    assert.equal(normalized.video_count, 1509);
    assert.equal(normalized.digg_count, 0);
    assert.deepEqual(normalized.user, {
        id: '107955',
        uniqueId: 'tiktok',
        nickname: 'TikTok',
        avatarThumb: 'thumb.webp',
        avatarMedium: 'medium.webp',
        avatarLarger: 'large.webp',
        signature: 'One TikTok can make a big impact',
        verified: true,
        privateAccount: false,
        region: 'US',
        language: 'en',
    });
});

test('preserves existing top-level profile fields when already present', () => {
    const normalized = normalizeTikTokProfileRecord({
        user_id: '123',
        unique_id: '@existing',
        nickname: 'Existing',
        avatar: 'avatar.webp',
        follower_count: 9,
        user: {
            id: '107955',
            uniqueId: 'tiktok',
            nickname: 'TikTok',
            avatarLarger: 'large.webp',
        },
        stats: {
            followerCount: 93950516,
        },
    });

    assert.equal(normalized.user_id, '123');
    assert.equal(normalized.unique_id, '@existing');
    assert.equal(normalized.nickname, 'Existing');
    assert.equal(normalized.avatar, 'avatar.webp');
    assert.equal(normalized.follower_count, 9);
});

test('coerces numeric nested user ids to strings', () => {
    const normalized = normalizeTikTokProfileRecord({
        user: {
            id: 107955,
            uniqueId: 'tiktok',
        },
    });

    assert.equal(normalized.user_id, '107955');
    assert.equal(normalized.unique_id, '@tiktok');
});

test('handles profiles without nested user or stats objects', () => {
    const normalized = normalizeTikTokProfileRecord({
        signature: 'Existing signature',
    });

    assert.equal(normalized.signature, 'Existing signature');
    assert.equal(normalized.user_id, undefined);
    assert.equal(normalized.unique_id, undefined);
    assert.equal(normalized.follower_count, undefined);
});
