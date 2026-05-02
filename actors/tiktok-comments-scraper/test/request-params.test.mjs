import assert from 'node:assert/strict';
import test from 'node:test';

import { buildTikTokCommentsParams, requireTikTokVideoUrl } from '../dist/request-params.js';

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
