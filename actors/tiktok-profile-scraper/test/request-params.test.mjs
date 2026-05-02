import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
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

test('rejects invalid unique_id username characters', () => {
    for (const value of ['tik tok', 'tik-tok', 'tik/tok']) {
        assert.throws(
            () => buildTikTokProfileParams({ unique_id: value }),
            /username must be 2 to 255 characters/,
        );
    }
});

test('rejects unique_id username outside length bounds', () => {
    for (const value of ['a', 'a'.repeat(256), '@a', `@${'a'.repeat(256)}`]) {
        assert.throws(
            () => buildTikTokProfileParams({ unique_id: value }),
            /username must be 2 to 255 characters/,
        );
    }
});

test('extracts unique_id from a TikTok profile URL', () => {
    assert.equal(
        normalizeTikTokUniqueId('https://www.tiktok.com/@tiktok?lang=en'),
        '@tiktok',
    );
});

test('rejects profile URLs with invalid username characters', () => {
    for (const value of ['https://www.tiktok.com/@tik-tok', 'https://www.tiktok.com/@tik%20tok']) {
        assert.throws(
            () => buildTikTokProfileParams({ unique_id: value }),
            /username must be 2 to 255 characters/,
        );
    }
});

test('rejects profile URLs with usernames outside length bounds', () => {
    for (const value of ['https://www.tiktok.com/@a', `https://www.tiktok.com/@${'a'.repeat(256)}`]) {
        assert.throws(
            () => buildTikTokProfileParams({ unique_id: value }),
            /username must be 2 to 255 characters/,
        );
    }
});

test('builds params from user_id', () => {
    assert.deepEqual(
        buildTikTokProfileParams({ user_id: ' 107955 ' }),
        { user_id: '107955' },
    );
});

test('rejects non-numeric user_id input', () => {
    assert.throws(
        () => buildTikTokProfileParams({ user_id: 'abc' }),
        /user_id must contain digits only/,
    );
});

test('rejects user_id input longer than user_id limit', () => {
    assert.throws(
        () => buildTikTokProfileParams({ user_id: '1'.repeat(31) }),
        /must be 30 digits or fewer/,
    );
});

test('builds params from numeric profile input as user_id', () => {
    assert.deepEqual(
        buildTikTokProfileParams({ profile: '107955' }),
        { user_id: '107955' },
    );
});

test('rejects digit-only profile input longer than user_id limit', () => {
    assert.throws(
        () => buildTikTokProfileParams({ profile: '1'.repeat(31) }),
        /must be 30 digits or fewer/,
    );
});

test('input schema rejects digit-only profile input longer than user_id limit', () => {
    const schema = JSON.parse(readFileSync(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
    const pattern = new RegExp(schema.properties.profile.pattern);

    assert.equal(pattern.test('1'.repeat(30)), true);
    assert.equal(pattern.test('1'.repeat(31)), false);
});

test('includes both lookup values when both are provided', () => {
    assert.deepEqual(
        buildTikTokProfileParams({ unique_id: 'tiktok', user_id: '107955' }),
        { unique_id: '@tiktok', user_id: '107955' },
    );
});

test('uses profile instead of legacy unique_id when both are provided', () => {
    assert.deepEqual(
        buildTikTokProfileParams({ profile: 'profileuser', unique_id: 'legacyuser' }),
        { unique_id: '@profileuser' },
    );
});

test('uses profile instead of legacy user_id when both are provided', () => {
    assert.deepEqual(
        buildTikTokProfileParams({ profile: 'profileuser', user_id: '107955' }),
        { unique_id: '@profileuser' },
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
