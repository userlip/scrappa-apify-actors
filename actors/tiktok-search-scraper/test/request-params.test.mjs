import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildTikTokSearchParams,
    formatTikTokSearchLookupForLog,
    normalizeTikTokSearchKeywords,
} from '../dist/request-params.js';

test('builds params from required keywords and optional controls', () => {
    assert.deepEqual(
        buildTikTokSearchParams({
            keywords: ' basketball highlights ',
            region: ' us ',
            count: 10,
            cursor: ' 0 ',
            publish_time: 7,
            sort_type: 1,
        }),
        {
            keywords: 'basketball highlights',
            region: 'US',
            count: 10,
            cursor: '0',
            publish_time: 7,
            sort_type: 1,
        },
    );
});

test('supports query as an alias for keywords', () => {
    assert.deepEqual(
        buildTikTokSearchParams({ query: '#skincare', count: 25 }),
        { keywords: '#skincare', count: 25 },
    );
});

test('keywords take precedence over query alias', () => {
    assert.deepEqual(
        buildTikTokSearchParams({ keywords: 'coffee', query: 'tea' }),
        { keywords: 'coffee' },
    );
});

test('normalizes repeated whitespace in keyword searches', () => {
    assert.equal(normalizeTikTokSearchKeywords(' viral   recipes '), 'viral recipes');
});

test('warns and omits invalid count values', () => {
    const warnings = [];
    const params = buildTikTokSearchParams(
        { keywords: 'basketball', count: 0 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { keywords: 'basketball' });
    assert.match(warnings[0], /count must be an integer between 1 and 50/);
});

test('warns and omits non-string region values', () => {
    const warnings = [];
    const params = buildTikTokSearchParams(
        { keywords: 'basketball', region: 123 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { keywords: 'basketball' });
    assert.match(warnings[0], /region must be a string/);
});

test('accepts numeric cursor values as strings', () => {
    assert.deepEqual(
        buildTikTokSearchParams({ keywords: 'basketball', cursor: 100 }),
        { keywords: 'basketball', cursor: '100' },
    );
});

test('warns and omits invalid publish_time and sort_type values', () => {
    const warnings = [];
    const params = buildTikTokSearchParams(
        { keywords: 'basketball', publish_time: -1, sort_type: 11 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { keywords: 'basketball' });
    assert.match(warnings[0], /publish_time must be an integer between 0 and 3650/);
    assert.match(warnings[1], /sort_type must be an integer between 0 and 10/);
});

test('rejects missing keyword input', () => {
    assert.throws(
        () => buildTikTokSearchParams({ keywords: ' ', query: '' }),
        /TikTok search keywords are required/,
    );
});

test('rejects malformed keyword input', () => {
    assert.throws(
        () => buildTikTokSearchParams({ keywords: 'basketball\nhighlights' }),
        /cannot contain tabs, line breaks, or control whitespace/,
    );
});

test('rejects control whitespace in keyword searches', () => {
    assert.throws(
        () => buildTikTokSearchParams({ keywords: 'basketball\vhighlights' }),
        /cannot contain tabs, line breaks, or control whitespace/,
    );
});

test('rejects invalid regions', () => {
    assert.throws(
        () => buildTikTokSearchParams({ keywords: 'basketball', region: 'USA-1' }),
        /region must be a 2 to 10 character/,
    );
});

test('formats lookup values for logs', () => {
    assert.equal(formatTikTokSearchLookupForLog({ keywords: 'basketball' }), 'basketball');
    assert.equal(formatTikTokSearchLookupForLog({ query: 'skincare' }), 'skincare');
});
