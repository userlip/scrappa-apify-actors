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

test('accepts numeric cursor values', () => {
    assert.deepEqual(
        buildTikTokHashtagPostsParams({ hashtag: 'fyp', cursor: 123 }),
        { challenge_name: 'fyp', cursor: '123' },
    );
});

test('warns and omits non-string, non-number cursor values', () => {
    const warnings = [];
    const params = buildTikTokHashtagPostsParams(
        { hashtag: 'fyp', cursor: true },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { challenge_name: 'fyp' });
    assert.match(warnings[0], /cursor must be a string or number/);
});

test('warns and omits non-finite numeric cursor values', () => {
    const warnings = [];
    const params = buildTikTokHashtagPostsParams(
        { hashtag: 'fyp', cursor: Number.NaN },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { challenge_name: 'fyp' });
    assert.match(warnings[0], /cursor must be a finite string or number/);
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
        () => buildTikTokHashtagPostsParams({ challenge_name: 'tik/tok' }),
        /TikTok hashtag must be 1 to 255 characters/,
    );
});

test('accepts Unicode and emoji hashtag names', () => {
    assert.deepEqual(
        buildTikTokHashtagPostsParams({ hashtag: '#booktok🧡' }),
        { challenge_name: 'booktok🧡' },
    );
    assert.deepEqual(
        buildTikTokHashtagPostsParams({ hashtag: '猫' }),
        { challenge_name: '猫' },
    );
});

test('treats domain-like hashtag values as hashtag names', () => {
    assert.deepEqual(
        buildTikTokHashtagPostsParams({ hashtag: 'example.tiktok.com' }),
        { challenge_name: 'example.tiktok.com' },
    );
});

test('formats lookup values for logs', () => {
    assert.equal(formatTikTokHashtagPostsLookupForLog({ hashtag: 'fyp' }), '#fyp');
    assert.equal(formatTikTokHashtagPostsLookupForLog({ hashtag: '33380' }), 'challenge_id:33380');
});

test('formats unknown lookup for logs', () => {
    assert.equal(formatTikTokHashtagPostsLookupForLog({ hashtag: ' ' }), 'unknown TikTok hashtag');
});

test('normalizes TikTok hashtag URLs into challenge_name values', () => {
    assert.equal(
        normalizeTikTokHashtag('https://www.tiktok.com/tag/tik.tok_123?lang=en'),
        'tik.tok_123',
    );
});

test('rejects malformed URL-like hashtag values', () => {
    assert.throws(
        () => normalizeTikTokHashtag('//www.tiktok.com/tag/fyp'),
        /A valid TikTok hashtag URL or hashtag name is required/,
    );
});
