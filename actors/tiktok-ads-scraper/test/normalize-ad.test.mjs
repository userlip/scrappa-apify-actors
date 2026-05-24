import assert from 'node:assert/strict';
import test from 'node:test';

import { normalizeTikTokAdRecord } from '../dist/normalize-ad.js';

test('normalizes nested Scrappa TikTok ad payload into top-level actor fields', () => {
    const normalized = normalizeTikTokAdRecord({
        id: '7221117041168252930',
        brand_name: 'Example brand',
        title: 'Example ad title',
        destination: 'https://example.com',
        play_url: 'https://cdn.example.com/video.mp4',
        cover_url: 'https://cdn.example.com/cover.jpg',
        stats: {
            like_count: 1200,
            comment_count: 42,
            share_count: 18,
        },
        advertiser: {
            id: 123,
            account_id: 456,
            name: 'Example advertiser',
        },
    });

    assert.equal(normalized.ad_id, '7221117041168252930');
    assert.equal(normalized.advertiser_id, '123');
    assert.equal(normalized.account_id, '456');
    assert.equal(normalized.advertiser_name, 'Example brand');
    assert.equal(normalized.account_name, 'Example advertiser');
    assert.equal(normalized.creative_text, 'Example ad title');
    assert.equal(normalized.landing_page, 'https://example.com');
    assert.equal(normalized.video_url, 'https://cdn.example.com/video.mp4');
    assert.equal(normalized.cover, 'https://cdn.example.com/cover.jpg');
    assert.deepEqual(normalized.media_urls, [
        'https://cdn.example.com/video.mp4',
        'https://cdn.example.com/cover.jpg',
    ]);
    assert.equal(normalized.like_count, 1200);
    assert.equal(normalized.comment_count, 42);
    assert.equal(normalized.share_count, 18);
});

test('preserves existing top-level fields when already present', () => {
    const normalized = normalizeTikTokAdRecord({
        ad_id: 'existing',
        advertiser_id: 'adv_existing',
        account_id: 'acct_existing',
        advertiser_name: 'Existing advertiser',
        account_name: 'Existing account',
        creative_text: 'Existing creative',
        landing_page_url: 'https://landing.example.com',
        video_url: 'https://video.example.com/video.mp4',
        cover: 'https://video.example.com/cover.jpg',
        like_count: 9,
        stats: {
            like_count: 1200,
        },
        advertiser: {
            id: 123,
            name: 'Nested advertiser',
        },
    });

    assert.equal(normalized.ad_id, 'existing');
    assert.equal(normalized.advertiser_id, 'adv_existing');
    assert.equal(normalized.account_id, 'acct_existing');
    assert.equal(normalized.advertiser_name, 'Existing advertiser');
    assert.equal(normalized.account_name, 'Existing account');
    assert.equal(normalized.creative_text, 'Existing creative');
    assert.equal(normalized.landing_page, 'https://landing.example.com');
    assert.equal(normalized.video_url, 'https://video.example.com/video.mp4');
    assert.equal(normalized.like_count, 9);
});

test('deduplicates media URLs and handles sparse records', () => {
    const normalized = normalizeTikTokAdRecord({
        media_url: 'https://cdn.example.com/video.mp4',
        video_url: 'https://cdn.example.com/video.mp4',
    });

    assert.equal(normalized.video_url, 'https://cdn.example.com/video.mp4');
    assert.deepEqual(normalized.media_urls, ['https://cdn.example.com/video.mp4']);
    assert.equal(normalized.ad_id, undefined);
});

test('normalizes fields from the live Scrappa TikTok ad response shape', () => {
    const normalized = normalizeTikTokAdRecord({
        id: '7543186103350427655',
        brand_name: 'TikTok',
        title: 'Promote your TikTok now!',
        cover_uri: 'https://cdn.example.com/cover.jpg',
        like: 5215200,
        comment: 1277551,
        share: 1388339,
        video_info: {
            play: 'https://cdn.example.com/play.mp4',
            wmplay: 'https://cdn.example.com/wmplay.mp4',
            cover: 'https://cdn.example.com/cover.jpg',
        },
    });

    assert.equal(normalized.ad_id, '7543186103350427655');
    assert.equal(normalized.advertiser_name, 'TikTok');
    assert.equal(normalized.creative_text, 'Promote your TikTok now!');
    assert.equal(normalized.video_url, 'https://cdn.example.com/play.mp4');
    assert.equal(normalized.cover, 'https://cdn.example.com/cover.jpg');
    assert.deepEqual(normalized.media_urls, [
        'https://cdn.example.com/play.mp4',
        'https://cdn.example.com/cover.jpg',
        'https://cdn.example.com/wmplay.mp4',
    ]);
    assert.equal(normalized.like_count, 5215200);
    assert.equal(normalized.comment_count, 1277551);
    assert.equal(normalized.share_count, 1388339);
});
