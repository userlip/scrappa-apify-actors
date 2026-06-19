import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildTikTokChallengeSearchRequests,
    formatTikTokChallengeSearchLookupForLog,
    normalizeTikTokChallengeSearchKeyword,
} from '../dist/request-params.js';

test('builds one request per keywords entry', () => {
    assert.deepEqual(
        buildTikTokChallengeSearchRequests({ keywords: [' cosplay ', 'fitness'], count: 10 }),
        [
            { keyword: 'cosplay', params: { keywords: 'cosplay', count: 10 } },
            { keyword: 'fitness', params: { keywords: 'fitness', count: 10 } },
        ],
    );
});

test('accepts a string keywords value for API compatibility', () => {
    assert.deepEqual(
        buildTikTokChallengeSearchRequests({ keywords: ' skincare ', count: 25 }),
        [{ keyword: 'skincare', params: { keywords: 'skincare', count: 25 } }],
    );
});

test('accepts legacy keyword input', () => {
    assert.deepEqual(
        buildTikTokChallengeSearchRequests({ keyword: ' makeup trends ', count: 5 }),
        [{ keyword: 'makeup trends', params: { keywords: 'makeup trends', count: 5 } }],
    );
});

test('ignores legacy keyword when keywords contains valid entries', () => {
    assert.deepEqual(
        buildTikTokChallengeSearchRequests({ keywords: ['coffee'], keyword: 'tea' }),
        [{ keyword: 'coffee', params: { keywords: 'coffee' } }],
    );
});

test('falls back to legacy keyword when keywords is empty', () => {
    assert.deepEqual(
        buildTikTokChallengeSearchRequests({ keywords: [], keyword: 'tea' }),
        [{ keyword: 'tea', params: { keywords: 'tea' } }],
    );
});

test('deduplicates keywords after normalization', () => {
    assert.deepEqual(
        buildTikTokChallengeSearchRequests({ keywords: ['cosplay', ' cosplay '] }),
        [{ keyword: 'cosplay', params: { keywords: 'cosplay' } }],
    );
});

test('normalizes repeated whitespace in keyword searches', () => {
    assert.equal(normalizeTikTokChallengeSearchKeyword(' viral   recipes '), 'viral recipes');
});

test('warns and omits invalid count values', () => {
    const warnings = [];
    const requests = buildTikTokChallengeSearchRequests(
        { keywords: ['cosplay'], count: 0 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(requests, [{ keyword: 'cosplay', params: { keywords: 'cosplay' } }]);
    assert.match(warnings[0], /count must be an integer between 1 and 50/);
});

test('warns and omits non-string keyword entries', () => {
    const warnings = [];
    const requests = buildTikTokChallengeSearchRequests(
        { keywords: ['cosplay', 123, null] },
        (message) => warnings.push(message),
    );

    assert.deepEqual(requests, [{ keyword: 'cosplay', params: { keywords: 'cosplay' } }]);
    assert.match(warnings[0], /keywords entries must be strings/);
});

test('warns and falls back when keywords has an invalid shape', () => {
    const warnings = [];
    const requests = buildTikTokChallengeSearchRequests(
        { keywords: { value: 'cosplay' }, keyword: 'fitness' },
        (message) => warnings.push(message),
    );

    assert.deepEqual(requests, [{ keyword: 'fitness', params: { keywords: 'fitness' } }]);
    assert.match(warnings[0], /keywords must be an array of strings/);
});

test('rejects missing keyword input', () => {
    assert.throws(
        () => buildTikTokChallengeSearchRequests({ keywords: [], keyword: '' }),
        /At least one TikTok challenge search keyword is required/,
    );
});

test('rejects malformed keyword input', () => {
    assert.throws(
        () => buildTikTokChallengeSearchRequests({ keywords: ['cosplay\nfitness'] }),
        /cannot contain tabs, line breaks, or control whitespace/,
    );
});

test('formats lookup values for logs', () => {
    assert.equal(formatTikTokChallengeSearchLookupForLog([]), 'unknown TikTok challenge search');
    assert.equal(
        formatTikTokChallengeSearchLookupForLog(buildTikTokChallengeSearchRequests({ keywords: ['cosplay'] })),
        'cosplay',
    );
    assert.equal(
        formatTikTokChallengeSearchLookupForLog(buildTikTokChallengeSearchRequests({ keywords: ['cosplay', 'fitness'] })),
        '2 TikTok challenge searches',
    );
});
