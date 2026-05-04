import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildTikTokUserPostsParams,
    formatTikTokUserPostsLookupForLog,
    normalizeTikTokUniqueId,
} from '../dist/request-params.js';

test('builds params from profile username', () => {
    assert.deepEqual(
        buildTikTokUserPostsParams({ profile: '@tiktok', count: 10, cursor: ' 0 ' }),
        { unique_id: '@tiktok', count: 10, cursor: '0' },
    );
});

test('builds params from TikTok profile URL', () => {
    assert.deepEqual(
        buildTikTokUserPostsParams({ profile: 'https://www.tiktok.com/@tiktok?lang=en' }),
        { unique_id: '@tiktok' },
    );
});

test('treats bare numeric profile values as user_id', () => {
    assert.deepEqual(
        buildTikTokUserPostsParams({ profile: '107955' }),
        { user_id: '107955' },
    );
});

test('falls back to explicit unique_id and ignores invalid user_id when unique_id is present', () => {
    assert.deepEqual(
        buildTikTokUserPostsParams({ unique_id: 'tiktok', user_id: 'abc' }),
        { unique_id: '@tiktok' },
    );
});

test('accepts explicit numeric user_id lookup', () => {
    assert.deepEqual(
        buildTikTokUserPostsParams({ user_id: ' 107955 ', count: 25 }),
        { user_id: '107955', count: 25 },
    );
});

test('warns and omits invalid count values', () => {
    const warnings = [];
    const params = buildTikTokUserPostsParams(
        { profile: '@tiktok', count: 0 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { unique_id: '@tiktok' });
    assert.match(warnings[0], /count must be an integer between 1 and 50/);
});

test('warns and omits non-numeric count values', () => {
    const warnings = [];
    const params = buildTikTokUserPostsParams(
        { profile: '@tiktok', count: 'abc' },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { unique_id: '@tiktok' });
    assert.match(warnings[0], /count must be an integer between 1 and 50/);
});

test('warns and omits non-string cursor values', () => {
    const warnings = [];
    const params = buildTikTokUserPostsParams(
        { profile: '@tiktok', cursor: 123 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { unique_id: '@tiktok' });
    assert.match(warnings[0], /cursor must be a string/);
});

test('omits empty cursor strings', () => {
    assert.deepEqual(
        buildTikTokUserPostsParams({ profile: '@tiktok', cursor: '   ' }),
        { unique_id: '@tiktok' },
    );
});

test('rejects missing lookup input', () => {
    assert.throws(
        () => buildTikTokUserPostsParams({ profile: ' ', user_id: '' }),
        /TikTok unique_id or user_id is required/,
    );
});

test('rejects non-TikTok profile URLs', () => {
    assert.throws(
        () => buildTikTokUserPostsParams({ profile: 'https://example.com/@tiktok' }),
        /must be on tiktok\.com/,
    );
});

test('rejects non-HTTPS TikTok profile URLs', () => {
    assert.throws(
        () => buildTikTokUserPostsParams({ profile: 'http://www.tiktok.com/@tiktok' }),
        /must use HTTPS/,
    );
});

test('rejects malformed TikTok usernames', () => {
    assert.throws(
        () => buildTikTokUserPostsParams({ unique_id: '@tik-tok' }),
        /TikTok username must be 2 to 255 characters/,
    );
});

test('formats lookup values for logs', () => {
    assert.equal(formatTikTokUserPostsLookupForLog({ profile: 'tiktok' }), '@tiktok');
    assert.equal(formatTikTokUserPostsLookupForLog({ profile: '107955' }), 'user_id:107955');
});

test('normalizes TikTok profile URLs into unique_id values', () => {
    assert.equal(
        normalizeTikTokUniqueId('https://www.tiktok.com/@tik.tok_123?lang=en'),
        '@tik.tok_123',
    );
});
