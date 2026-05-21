import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
    buildTikTokVideoParams,
    formatTikTokVideoLookupForLog,
    requireTikTokVideoLookup,
    resolveTikTokVideoLookups,
    resolveTikTokVideoRequests,
} from '../dist/request-params.js';

const url = 'https://www.tiktok.com/@tiktok/video/7568510388342443294';

test('builds params with required URL only by default', () => {
    const warnings = [];
    const params = buildTikTokVideoParams(url, { urls: [url] }, (message) => warnings.push(message));

    assert.deepEqual(params, { url });
    assert.deepEqual(warnings, []);
});

test('adds hd only when true', () => {
    assert.deepEqual(
        buildTikTokVideoParams(url, { hd: true }),
        { url, hd: 1 },
    );
    assert.deepEqual(
        buildTikTokVideoParams(url, { hd: false }),
        { url },
    );
});

test('warns and omits non-boolean hd values', () => {
    const warnings = [];
    const params = buildTikTokVideoParams(url, { hd: 'true' }, (message) => warnings.push(message));

    assert.deepEqual(params, { url });
    assert.equal(warnings.length, 1);
    assert.match(warnings[0], /hd must be a boolean/);
});

test('resolves batch URL input and preserves duplicate requests', () => {
    assert.deepEqual(
        resolveTikTokVideoLookups({ urls: [` ${url} `, url] }),
        [url, url],
    );
});

test('falls back to legacy url input', () => {
    assert.deepEqual(
        resolveTikTokVideoLookups({ url: ` ${url} ` }),
        [url],
    );
});

test('prefers batch urls over legacy url', () => {
    const secondUrl = 'https://www.tiktok.com/@tiktok/video/1234567890123456789';

    assert.deepEqual(
        resolveTikTokVideoLookups({ urls: [secondUrl], url }),
        [secondUrl],
    );
});

test('warns and skips invalid non-string batch entries', () => {
    const warnings = [];
    const lookups = resolveTikTokVideoLookups(
        { urls: [url, 123, ' '] },
        (message) => warnings.push(message),
    );

    assert.deepEqual(lookups, [url]);
    assert.equal(warnings.length, 2);
    assert.match(warnings[0], /urls\[1] must be a string/);
    assert.match(warnings[1], /urls\[2] is empty/);
});

test('keeps invalid string batch entries as failed lookup requests', () => {
    const warnings = [];
    const lookups = resolveTikTokVideoRequests(
        { urls: ['not-a-url', url] },
        (message) => warnings.push(message),
    );

    assert.deepEqual(lookups, [
        {
            url: 'not-a-url',
            validationError: 'A valid TikTok video URL, short URL, photo URL, or video ID is required',
        },
        { url },
    ]);
    assert.equal(warnings.length, 1);
    assert.match(warnings[0], /urls\[0] is invalid/);
    assert.deepEqual(resolveTikTokVideoLookups({ urls: ['not-a-url', url] }, () => {}), [url]);
});

test('accepts canonical TikTok video URLs', () => {
    assert.doesNotThrow(() => requireTikTokVideoLookup(url));
    assert.doesNotThrow(() => requireTikTokVideoLookup(`${url}?lang=en&utm_source=test`));
});

test('accepts TikTok photo URLs, short URLs, and raw video IDs', () => {
    assert.doesNotThrow(() => requireTikTokVideoLookup('https://www.tiktok.com/@tiktok/photo/7568510388342443294'));
    assert.doesNotThrow(() => requireTikTokVideoLookup('https://vm.tiktok.com/ZGeqDY4yL/'));
    assert.doesNotThrow(() => requireTikTokVideoLookup('https://vt.tiktok.com/ZGeqDY4yL/'));
    assert.doesNotThrow(() => requireTikTokVideoLookup('https://tiktok.com/t/ZGeqDY4yL/'));
    assert.doesNotThrow(() => requireTikTokVideoLookup('7568510388342443294'));
});

test('rejects non-content TikTok URLs before calling Scrappa', () => {
    assert.throws(
        () => requireTikTokVideoLookup('https://www.tiktok.com/@tiktok'),
        /video URL, short URL, photo URL, or video ID/,
    );
    assert.throws(
        () => requireTikTokVideoLookup('https://www.tiktok.com/tag/example'),
        /video URL, short URL, photo URL, or video ID/,
    );
    assert.throws(
        () => requireTikTokVideoLookup('https://www.tiktok.com/privacy'),
        /video URL, short URL, photo URL, or video ID/,
    );
    assert.throws(
        () => requireTikTokVideoLookup('https://www.tiktok.com/settings'),
        /video URL, short URL, photo URL, or video ID/,
    );
    assert.throws(
        () => requireTikTokVideoLookup('https://www.tiktok.com/ZGeqDY4yL'),
        /video URL, short URL, photo URL, or video ID/,
    );
});

test('rejects non-TikTok URLs', () => {
    assert.throws(
        () => requireTikTokVideoLookup('https://example.com/@tiktok/video/7568510388342443294'),
        /TikTok URL is required/,
    );
});

test('rejects non-HTTPS TikTok URLs', () => {
    assert.throws(
        () => requireTikTokVideoLookup('http://www.tiktok.com/@tiktok/video/7568510388342443294'),
        /must use HTTPS/,
    );
});

test('formats TikTok URLs for logs without query parameters or fragments', () => {
    assert.equal(
        formatTikTokVideoLookupForLog(`${url}?utm_source=test&token=abc#comments`),
        url,
    );
    assert.equal(formatTikTokVideoLookupForLog('7568510388342443294'), 'video_id:7568510388342443294');
});

test('requires at least one lookup', () => {
    assert.throws(
        () => resolveTikTokVideoLookups({ urls: [], url: '' }),
        /At least one TikTok video URL is required/,
    );
});

test('input schema supports batch urls and legacy url', () => {
    const schema = JSON.parse(readFileSync(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
    const itemPattern = new RegExp(schema.properties.urls.items.pattern);
    const legacyPattern = new RegExp(schema.properties.url.pattern);

    assert.equal(itemPattern.test(url), true);
    assert.equal(itemPattern.test('https://vm.tiktok.com/ZGeqDY4yL/'), true);
    assert.equal(itemPattern.test('https://vt.tiktok.com/ZGeqDY4yL/'), true);
    assert.equal(itemPattern.test('7568510388342443294'), true);
    assert.equal(legacyPattern.test(url), true);
    assert.equal(itemPattern.test('https://example.com/@tiktok/video/7568510388342443294'), false);
    assert.equal(itemPattern.test('https://www.tiktok.com/privacy'), false);
    assert.equal(itemPattern.test('https://www.tiktok.com/ZGeqDY4yL'), false);
});
