import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildTikTokCommentRepliesParams,
    buildTikTokCommentsParams,
    extractTikTokVideoId,
    formatTikTokVideoUrlForLog,
    requireTikTokVideoUrl,
    resolveMaxRepliesPerComment,
    shouldIncludeTikTokCommentReplies,
} from '../dist/request-params.js';

const url = 'https://www.tiktok.com/@tiktok/video/7568510388342443294';

test('builds params with valid pagination controls', () => {
    const warnings = [];
    const params = buildTikTokCommentsParams(
        { url, count: 50, cursor: '1700000000000' },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, {
        url,
        count: 50,
        cursor: '1700000000000',
    });
    assert.deepEqual(warnings, []);
});

test('omits invalid counts to avoid Scrappa validation failures', () => {
    for (const count of [0, 51, 1.5, '10', Number.NaN, null]) {
        const warnings = [];
        const params = buildTikTokCommentsParams(
            { url, count },
            (message) => warnings.push(message),
        );

        assert.deepEqual(params, { url });
        assert.equal(warnings.length, 1);
        assert.match(warnings[0], /count must be an integer between 1 and 50/);
    }
});

test('trims string cursor and omits empty cursor', () => {
    assert.deepEqual(
        buildTikTokCommentsParams({ url, cursor: ' 12345 ' }),
        { url, cursor: '12345' },
    );
    assert.deepEqual(
        buildTikTokCommentsParams({ url, cursor: ' ' }),
        { url },
    );
});

test('warns and omits non-string cursor values', () => {
    const warnings = [];
    const params = buildTikTokCommentsParams(
        { url, cursor: 12345 },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { url });
    assert.equal(warnings.length, 1);
    assert.match(warnings[0], /cursor must be a string/);
});

test('accepts canonical TikTok video URLs', () => {
    assert.doesNotThrow(() => requireTikTokVideoUrl(url));
    assert.doesNotThrow(() => requireTikTokVideoUrl(`${url}?lang=en&utm_source=test`));
});

test('rejects non-video TikTok URLs before calling Scrappa', () => {
    assert.throws(
        () => requireTikTokVideoUrl('https://www.tiktok.com/@tiktok'),
        /must use the format/,
    );
    assert.throws(
        () => requireTikTokVideoUrl('https://www.tiktok.com/tag/example'),
        /must use the format/,
    );
});

test('rejects non-TikTok URLs', () => {
    assert.throws(
        () => requireTikTokVideoUrl('https://example.com/@tiktok/video/7568510388342443294'),
        /TikTok video URL is required/,
    );
});

test('rejects non-HTTPS TikTok video URLs', () => {
    assert.throws(
        () => requireTikTokVideoUrl('http://www.tiktok.com/@tiktok/video/7568510388342443294'),
        /must use HTTPS/,
    );
});

test('formats TikTok video URLs for logs without query parameters or fragments', () => {
    assert.equal(
        formatTikTokVideoUrlForLog(`${url}?utm_source=test&token=abc#comments`),
        url,
    );
});

test('extracts TikTok video ID from canonical URLs', () => {
    assert.equal(extractTikTokVideoId(`${url}?lang=en`), '7568510388342443294');
});

test('requires explicit reply collection opt-in', () => {
    assert.equal(shouldIncludeTikTokCommentReplies({ url }), false);
    assert.equal(shouldIncludeTikTokCommentReplies({ url, includeReplies: false }), false);
    assert.equal(shouldIncludeTikTokCommentReplies({ url, includeReplies: true }), true);
});

test('resolves max replies per comment with bounds', () => {
    assert.equal(resolveMaxRepliesPerComment({ url }), 50);
    assert.equal(resolveMaxRepliesPerComment({ url, maxRepliesPerComment: 125 }), 125);

    const warnings = [];
    assert.equal(
        resolveMaxRepliesPerComment({ url, maxRepliesPerComment: 501 }, (message) => warnings.push(message)),
        50,
    );
    assert.equal(warnings.length, 1);
    assert.match(warnings[0], /maxRepliesPerComment must be an integer between 1 and 500/);
});

test('builds reply params with parent comment, video ID, count, and cursor', () => {
    const warnings = [];
    const params = buildTikTokCommentRepliesParams(
        {
            comment_id: ' 7093219663211053829 ',
            video_id: ' 7568510388342443294 ',
            count: 50,
            cursor: ' 1700000000000 ',
        },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, {
        comment_id: '7093219663211053829',
        video_id: '7568510388342443294',
        count: 50,
        cursor: '1700000000000',
    });
    assert.deepEqual(warnings, []);
});

test('omits invalid reply pagination controls', () => {
    const warnings = [];
    const params = buildTikTokCommentRepliesParams(
        {
            comment_id: '7093219663211053829',
            video_id: '',
            count: 51,
            cursor: 123,
        },
        (message) => warnings.push(message),
    );

    assert.deepEqual(params, { comment_id: '7093219663211053829' });
    assert.equal(warnings.length, 2);
    assert.match(warnings[0], /reply count must be an integer between 1 and 50/);
    assert.match(warnings[1], /reply cursor must be a string/);
});

test('requires comment ID for reply params', () => {
    assert.throws(
        () => buildTikTokCommentRepliesParams({ comment_id: ' ' }),
        /comment_id is required/,
    );
});
