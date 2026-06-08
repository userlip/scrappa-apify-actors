import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildTikTokFollowingParams,
    formatTikTokFollowingLookupForLog,
    normalizeTikTokUniqueId,
} from '../dist/request-params.js';

test('builds params from profile username', () => {
    assert.deepEqual(
        buildTikTokFollowingParams({ profile: '@tiktok', count: 10, time: ' 0 ' }),
        { unique_id: '@tiktok', count: 10, time: '0' },
    );
});

test('builds params from TikTok profile URL', () => {
    assert.deepEqual(
        buildTikTokFollowingParams({ profile: 'https://www.tiktok.com/@tiktok?lang=en' }),
        { unique_id: '@tiktok' },
    );
});

test('treats bare numeric profile values as user_id', () => {
    assert.deepEqual(
        buildTikTokFollowingParams({ profile: '107955' }),
        { user_id: '107955' },
    );
});

test('falls back to explicit unique_id and ignores invalid user_id when unique_id is present', () => {
    assert.deepEqual(
        buildTikTokFollowingParams({ unique_id: 'tiktok', user_id: 'abc' }),
        { unique_id: '@tiktok' },
    );
});

test('accepts explicit numeric user_id lookup', () => {
    assert.deepEqual(
        buildTikTokFollowingParams({ user_id: ' 107955 ', count: 25 }),
        { user_id: '107955', count: 25 },
    );
});

test('accepts high count values for multi-page runs without an actor-side maximum', () => {
    assert.deepEqual(
        buildTikTokFollowingParams({ profile: '@tiktok', count: 100000 }),
        { unique_id: '@tiktok', count: 100000 },
    );
});

test('warns and omits invalid count values', () => {
    const warnings = [];
    const params = buildTikTokFollowingParams(
        { profile: '@tiktok', count: 0 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { unique_id: '@tiktok' });
    assert.match(warnings[0], /count must be a positive integer/);
});

test('warns and omits non-numeric count values', () => {
    const warnings = [];
    const params = buildTikTokFollowingParams(
        { profile: '@tiktok', count: 'abc' },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { unique_id: '@tiktok' });
    assert.match(warnings[0], /count must be a positive integer/);
});

test('accepts cursor as a compatibility alias for time', () => {
    assert.deepEqual(
        buildTikTokFollowingParams({ profile: '@tiktok', cursor: '123' }),
        { unique_id: '@tiktok', time: '123' },
    );
});

test('warns and omits invalid time values', () => {
    const warnings = [];
    const params = buildTikTokFollowingParams(
        { profile: '@tiktok', time: 'abc' },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { unique_id: '@tiktok' });
    assert.match(warnings[0], /time must contain digits only/);
});

test('omits empty time strings', () => {
    assert.deepEqual(
        buildTikTokFollowingParams({ profile: '@tiktok', time: '   ' }),
        { unique_id: '@tiktok' },
    );
});

test('rejects missing lookup input', () => {
    assert.throws(
        () => buildTikTokFollowingParams({ profile: ' ', user_id: '' }),
        /TikTok unique_id or user_id is required/,
    );
});

test('rejects non-TikTok profile URLs', () => {
    assert.throws(
        () => buildTikTokFollowingParams({ profile: 'https://example.com/@tiktok' }),
        /must be on tiktok\.com/,
    );
});

test('rejects non-HTTPS TikTok profile URLs', () => {
    assert.throws(
        () => buildTikTokFollowingParams({ profile: 'http://www.tiktok.com/@tiktok' }),
        /must use HTTPS/,
    );
});

test('rejects malformed TikTok usernames', () => {
    assert.throws(
        () => buildTikTokFollowingParams({ unique_id: '@tik-tok' }),
        /TikTok username must be 2 to 255 characters/,
    );
});

test('formats lookup values for logs', () => {
    assert.equal(formatTikTokFollowingLookupForLog({ profile: 'tiktok' }), '@tiktok');
    assert.equal(formatTikTokFollowingLookupForLog({ profile: '107955' }), 'user_id:107955');
});

test('normalizes TikTok profile URLs into unique_id values', () => {
    assert.equal(
        normalizeTikTokUniqueId('https://www.tiktok.com/@tik.tok_123?lang=en'),
        '@tik.tok_123',
    );
});
