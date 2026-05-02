import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildTikTokProfileParams,
    formatTikTokProfileLookupForLog,
    normalizeTikTokUniqueId,
} from '../dist/request-params.js';

test('builds params from a username with @', () => {
    assert.deepEqual(
        buildTikTokProfileParams({ unique_id: '@tiktok' }),
        { unique_id: '@tiktok' },
    );
});

test('builds params from required profile input', () => {
    assert.deepEqual(
        buildTikTokProfileParams({ profile: '@tiktok' }),
        { unique_id: '@tiktok' },
    );
});

test('adds @ to usernames without one', () => {
    assert.deepEqual(
        buildTikTokProfileParams({ profile: 'tiktok' }),
        { unique_id: '@tiktok' },
    );
});

test('extracts unique_id from a TikTok profile URL', () => {
    assert.equal(
        normalizeTikTokUniqueId('https://www.tiktok.com/@tiktok?lang=en'),
        '@tiktok',
    );
});

test('builds params from user_id', () => {
    assert.deepEqual(
        buildTikTokProfileParams({ user_id: ' 107955 ' }),
        { user_id: '107955' },
    );
});

test('builds params from numeric profile input as user_id', () => {
    assert.deepEqual(
        buildTikTokProfileParams({ profile: '107955' }),
        { user_id: '107955' },
    );
});

test('includes both lookup values when both are provided', () => {
    assert.deepEqual(
        buildTikTokProfileParams({ unique_id: 'tiktok', user_id: '107955' }),
        { unique_id: '@tiktok', user_id: '107955' },
    );
});

test('warns and omits non-string lookup values', () => {
    const warnings = [];
    const params = buildTikTokProfileParams(
        { unique_id: 123, user_id: '107955' },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { user_id: '107955' });
    assert.equal(warnings.length, 1);
    assert.match(warnings[0], /unique_id must be a string/);
});

test('requires unique_id or user_id', () => {
    assert.throws(
        () => buildTikTokProfileParams({ unique_id: ' ', user_id: '' }),
        /TikTok unique_id or user_id is required/,
    );
});

test('rejects non-TikTok profile URLs', () => {
    assert.throws(
        () => buildTikTokProfileParams({ unique_id: 'https://example.com/@tiktok' }),
        /must be on tiktok\.com/,
    );
});

test('rejects non-HTTPS TikTok profile URLs', () => {
    assert.throws(
        () => buildTikTokProfileParams({ unique_id: 'http://www.tiktok.com/@tiktok' }),
        /must use HTTPS/,
    );
});

test('rejects non-profile TikTok URLs', () => {
    assert.throws(
        () => buildTikTokProfileParams({ unique_id: 'https://www.tiktok.com/tag/example' }),
        /must use the format/,
    );
});

test('rejects malformed URL-like profile values', () => {
    for (const value of ['https://', 'https://www.tiktok.com', 'www.tiktok.com/@tiktok', '//www.tiktok.com/@tiktok']) {
        assert.throws(
            () => buildTikTokProfileParams({ profile: value }),
            /valid TikTok profile URL or username|required|must use the format/,
        );
    }
});

test('formats lookup values for logs', () => {
    assert.equal(formatTikTokProfileLookupForLog({ profile: 'tiktok' }), '@tiktok');
    assert.equal(formatTikTokProfileLookupForLog({ profile: '107955' }), 'user_id:107955');
});
