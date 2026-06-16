import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildTikTokMusicPostsRequests,
    formatTikTokMusicPostsLookupForLog,
    normalizeTikTokMusicId,
} from '../dist/request-params.js';

test('builds one request per musicIds entry', () => {
    assert.deepEqual(
        buildTikTokMusicPostsRequests({ musicIds: ['7002634556977908485', '7002634556977908486'], count: 10, cursor: ' 0 ' }),
        [
            { musicId: '7002634556977908485', params: { music_id: '7002634556977908485', count: 10, cursor: '0' } },
            { musicId: '7002634556977908486', params: { music_id: '7002634556977908486', count: 10, cursor: '0' } },
        ],
    );
});

test('accepts legacy music_id input', () => {
    assert.deepEqual(
        buildTikTokMusicPostsRequests({ music_id: ' 7002634556977908485 ', count: 25 }),
        [{ musicId: '7002634556977908485', params: { music_id: '7002634556977908485', count: 25 } }],
    );
});

test('deduplicates musicIds and legacy music_id', () => {
    assert.deepEqual(
        buildTikTokMusicPostsRequests({ musicIds: ['7002634556977908485'], music_id: '7002634556977908485' }),
        [{ musicId: '7002634556977908485', params: { music_id: '7002634556977908485' } }],
    );
});

test('accepts safe integer music ID values', () => {
    assert.deepEqual(
        buildTikTokMusicPostsRequests({ musicIds: [12345], music_id: 67890 }),
        [
            { musicId: '12345', params: { music_id: '12345' } },
            { musicId: '67890', params: { music_id: '67890' } },
        ],
    );
});

test('warns and omits invalid count values', () => {
    const warnings = [];
    const requests = buildTikTokMusicPostsRequests(
        { musicIds: ['7002634556977908485'], count: 0 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(requests, [{ musicId: '7002634556977908485', params: { music_id: '7002634556977908485' } }]);
    assert.match(warnings[0], /count must be an integer between 1 and 50/);
});

test('accepts numeric cursor values', () => {
    assert.deepEqual(
        buildTikTokMusicPostsRequests({ musicIds: ['7002634556977908485'], cursor: 123 }),
        [{ musicId: '7002634556977908485', params: { music_id: '7002634556977908485', cursor: '123' } }],
    );
});

test('warns and omits non-string, non-number cursor values', () => {
    const warnings = [];
    const requests = buildTikTokMusicPostsRequests(
        { musicIds: ['7002634556977908485'], cursor: true },
        (message) => warnings.push(message),
    );

    assert.deepEqual(requests, [{ musicId: '7002634556977908485', params: { music_id: '7002634556977908485' } }]);
    assert.match(warnings[0], /cursor must be a string or number/);
});

test('rejects missing music IDs', () => {
    assert.throws(
        () => buildTikTokMusicPostsRequests({ musicIds: [], music_id: '' }),
        /At least one TikTok music_id is required/,
    );
});

test('rejects non-numeric music IDs', () => {
    assert.throws(
        () => buildTikTokMusicPostsRequests({ musicIds: ['track-name'] }),
        /TikTok music_id must contain digits only/,
    );
});

test('rejects music IDs longer than 100 digits', () => {
    assert.throws(
        () => normalizeTikTokMusicId('1'.repeat(101)),
        /TikTok music_id must be 100 digits or fewer/,
    );
});

test('warns and falls back when musicIds is not an array', () => {
    const warnings = [];
    const requests = buildTikTokMusicPostsRequests(
        { musicIds: '7002634556977908485', music_id: '7002634556977908486' },
        (message) => warnings.push(message),
    );

    assert.deepEqual(requests, [{ musicId: '7002634556977908486', params: { music_id: '7002634556977908486' } }]);
    assert.match(warnings[0], /musicIds must be an array/);
});

test('formats lookup values for logs', () => {
    assert.equal(formatTikTokMusicPostsLookupForLog([]), 'unknown TikTok music');
    assert.equal(
        formatTikTokMusicPostsLookupForLog(buildTikTokMusicPostsRequests({ musicIds: ['7002634556977908485'] })),
        'music_id:7002634556977908485',
    );
    assert.equal(
        formatTikTokMusicPostsLookupForLog(buildTikTokMusicPostsRequests({ musicIds: ['1', '2'] })),
        '2 TikTok music IDs',
    );
});
