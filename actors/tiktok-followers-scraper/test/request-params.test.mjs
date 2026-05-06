import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildTikTokFollowersParams,
    formatTikTokFollowersLookupForLog,
    normalizeTikTokUniqueId,
} from '../dist/request-params.js';

test('builds params from profile username', () => {
    assert.deepEqual(
        buildTikTokFollowersParams({ profile: '@tiktok', count: 10, time: ' 0 ' }),
        { unique_id: '@tiktok', count: 10, time: '0' },
    );
});

test('builds params from TikTok profile URL', () => {
    assert.deepEqual(
        buildTikTokFollowersParams({ profile: 'https://www.tiktok.com/@tiktok?lang=en' }),
        { unique_id: '@tiktok' },
    );
});

test('treats bare numeric profile values as user_id', () => {
    assert.deepEqual(
        buildTikTokFollowersParams({ profile: '107955' }),
        { user_id: '107955' },
    );
});

test('falls back to explicit unique_id and ignores invalid user_id when unique_id is present', () => {
    assert.deepEqual(
        buildTikTokFollowersParams({ unique_id: 'tiktok', user_id: 'abc' }),
        { unique_id: '@tiktok' },
    );
});

test('accepts explicit numeric user_id lookup', () => {
    assert.deepEqual(
        buildTikTokFollowersParams({ user_id: ' 107955 ', count: 25 }),
        { user_id: '107955', count: 25 },
    );
});

test('warns and omits invalid count values', () => {
    const warnings = [];
    const params = buildTikTokFollowersParams(
        { profile: '@tiktok', count: 0 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { unique_id: '@tiktok' });
    assert.match(warnings[0], /count must be an integer between 1 and 50/);
});

test('warns and omits non-numeric count values', () => {
    const warnings = [];
    const params = buildTikTokFollowersParams(
        { profile: '@tiktok', count: 'abc' },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { unique_id: '@tiktok' });
    assert.match(warnings[0], /count must be an integer between 1 and 50/);
});

test('accepts cursor as a compatibility alias for time', () => {
    assert.deepEqual(
        buildTikTokFollowersParams({ profile: '@tiktok', cursor: '123' }),
        { unique_id: '@tiktok', time: '123' },
    );
});

test('warns and omits invalid time values', () => {
    const warnings = [];
    const params = buildTikTokFollowersParams(
        { profile: '@tiktok', time: 'abc' },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { unique_id: '@tiktok' });
    assert.match(warnings[0], /time must contain digits only/);
});

test('omits empty time strings', () => {
    assert.deepEqual(
        buildTikTokFollowersParams({ profile: '@tiktok', time: '   ' }),
        { unique_id: '@tiktok' },
    );
});

test('rejects missing lookup input', () => {
    assert.throws(
        () => buildTikTokFollowersParams({ profile: ' ', user_id: '' }),
        /TikTok unique_id or user_id is required/,
    );
});

test('rejects non-TikTok profile URLs', () => {
    assert.throws(
        () => buildTikTokFollowersParams({ profile: 'https://example.com/@tiktok' }),
        /must be on tiktok\.com/,
    );
});

test('rejects non-HTTPS TikTok profile URLs', () => {
    assert.throws(
        () => buildTikTokFollowersParams({ profile: 'http://www.tiktok.com/@tiktok' }),
        /must use HTTPS/,
    );
});

test('rejects malformed TikTok usernames', () => {
    assert.throws(
        () => buildTikTokFollowersParams({ unique_id: '@tik-tok' }),
        /TikTok username must be 2 to 255 characters/,
    );
});

test('formats lookup values for logs', () => {
    assert.equal(formatTikTokFollowersLookupForLog({ profile: 'tiktok' }), '@tiktok');
    assert.equal(formatTikTokFollowersLookupForLog({ profile: '107955' }), 'user_id:107955');
});

test('normalizes TikTok profile URLs into unique_id values', () => {
    assert.equal(
        normalizeTikTokUniqueId('https://www.tiktok.com/@tik.tok_123?lang=en'),
        '@tik.tok_123',
    );
});
