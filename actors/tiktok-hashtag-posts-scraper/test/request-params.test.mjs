import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildTikTokHashtagPostsParams,
    formatTikTokHashtagPostsLookupForLog,
    normalizeTikTokHashtag,
} from '../dist/request-params.js';

test('builds params from hashtag name', () => {
    assert.deepEqual(
        buildTikTokHashtagPostsParams({ hashtag: '#fyp', count: 10, cursor: ' 0 ' }),
        { challenge_name: 'fyp', count: 10, cursor: '0' },
    );
});

test('builds params from TikTok hashtag URL', () => {
    assert.deepEqual(
        buildTikTokHashtagPostsParams({ hashtag: 'https://www.tiktok.com/tag/cosplay?lang=en' }),
        { challenge_name: 'cosplay' },
    );
});

test('treats bare numeric hashtag values as challenge_id', () => {
    assert.deepEqual(
        buildTikTokHashtagPostsParams({ hashtag: '33380' }),
        { challenge_id: '33380' },
    );
});

test('accepts explicit challenge_name and ignores invalid challenge_id when challenge_name is present', () => {
    assert.deepEqual(
        buildTikTokHashtagPostsParams({ challenge_name: 'BookTok', challenge_id: 'abc' }),
        { challenge_name: 'BookTok' },
    );
});

test('accepts explicit numeric challenge_id lookup', () => {
    assert.deepEqual(
        buildTikTokHashtagPostsParams({ challenge_id: ' 33380 ', count: 25 }),
        { challenge_id: '33380', count: 25 },
    );
});

test('adds normalized region when valid', () => {
    assert.deepEqual(
        buildTikTokHashtagPostsParams({ hashtag: 'fyp', region: ' us ' }),
        { challenge_name: 'fyp', region: 'US' },
    );
});

test('warns and omits invalid count values', () => {
    const warnings = [];
    const params = buildTikTokHashtagPostsParams(
        { hashtag: 'fyp', count: 0 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { challenge_name: 'fyp' });
    assert.match(warnings[0], /count must be an integer between 1 and 50/);
});

test('warns and omits non-numeric count values', () => {
    const warnings = [];
    const params = buildTikTokHashtagPostsParams(
        { hashtag: 'fyp', count: 'abc' },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { challenge_name: 'fyp' });
    assert.match(warnings[0], /count must be an integer between 1 and 50/);
});

test('warns and omits non-string cursor values', () => {
    const warnings = [];
    const params = buildTikTokHashtagPostsParams(
        { hashtag: 'fyp', cursor: 123 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { challenge_name: 'fyp' });
    assert.match(warnings[0], /cursor must be a string/);
});

test('omits empty cursor strings', () => {
    assert.deepEqual(
        buildTikTokHashtagPostsParams({ hashtag: 'fyp', cursor: '   ' }),
        { challenge_name: 'fyp' },
    );
});

test('rejects missing lookup input', () => {
    assert.throws(
        () => buildTikTokHashtagPostsParams({ hashtag: ' ', challenge_id: '' }),
        /TikTok challenge_id or challenge_name is required/,
    );
});

test('rejects non-TikTok hashtag URLs', () => {
    assert.throws(
        () => buildTikTokHashtagPostsParams({ hashtag: 'https://example.com/tag/fyp' }),
        /must be on tiktok\.com/,
    );
});

test('rejects non-HTTPS TikTok hashtag URLs', () => {
    assert.throws(
        () => buildTikTokHashtagPostsParams({ hashtag: 'http://www.tiktok.com/tag/fyp' }),
        /must use HTTPS/,
    );
});

test('rejects malformed TikTok hashtags', () => {
    assert.throws(
        () => buildTikTokHashtagPostsParams({ challenge_name: 'tik-tok' }),
        /TikTok hashtag must be 1 to 255 characters/,
    );
});

test('formats lookup values for logs', () => {
    assert.equal(formatTikTokHashtagPostsLookupForLog({ hashtag: 'fyp' }), '#fyp');
    assert.equal(formatTikTokHashtagPostsLookupForLog({ hashtag: '33380' }), 'challenge_id:33380');
});

test('normalizes TikTok hashtag URLs into challenge_name values', () => {
    assert.equal(
        normalizeTikTokHashtag('https://www.tiktok.com/tag/tik.tok_123?lang=en'),
        'tik.tok_123',
    );
});
