import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
    buildTikTokAdParams,
    extractTikTokAdId,
    formatTikTokAdLookupForLog,
    normalizeTikTokAdUrl,
    requireTikTokAdUrl,
    resolveTikTokAdRequests,
    resolveTikTokAdUrls,
} from '../dist/request-params.js';

const url = 'https://ads.tiktok.com/business/creativecenter/topads/7543186103350427655/pc/en';
const shortUrl = 'https://ads.tiktok.com/business/creativecenter/topads/7221117041168252930/';

test('builds Scrappa params with the required URL only', () => {
    assert.deepEqual(buildTikTokAdParams(url), { url });
});

test('normalizes Creative Center ad URLs for Scrappa and logs', () => {
    assert.equal(
        normalizeTikTokAdUrl(`${url}?period=30&region=US#details`),
        url,
    );
    assert.equal(extractTikTokAdId(url), '7543186103350427655');
    assert.equal(formatTikTokAdLookupForLog(url), 'ad_id:7543186103350427655');
    assert.equal(extractTikTokAdId(shortUrl), '7221117041168252930');
});

test('resolves batch URL input and preserves duplicate requests', () => {
    assert.deepEqual(
        resolveTikTokAdUrls({ urls: [` ${url} `, url] }),
        [url, url],
    );
});

test('falls back to legacy url input', () => {
    assert.deepEqual(
        resolveTikTokAdUrls({ url: ` ${url} ` }),
        [url],
    );
});

test('prefers batch urls over legacy url', () => {
    const secondUrl = 'https://ads.tiktok.com/business/creativecenter/topads/1234567890123456789/pc/en';

    assert.deepEqual(
        resolveTikTokAdUrls({ urls: [secondUrl], url }),
        [secondUrl],
    );
});

test('warns and skips invalid non-string batch entries', () => {
    const warnings = [];
    const lookups = resolveTikTokAdUrls(
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
    const lookups = resolveTikTokAdRequests(
        { urls: ['not-a-url', url] },
        (message) => warnings.push(message),
    );

    assert.deepEqual(lookups, [
        {
            url: 'not-a-url',
            validationError: 'A valid TikTok Creative Center ad URL is required',
        },
        { url },
    ]);
    assert.equal(warnings.length, 1);
    assert.match(warnings[0], /urls\[0] is invalid/);
    assert.deepEqual(resolveTikTokAdUrls({ urls: ['not-a-url', url] }, () => {}), [url]);
});

test('accepts TikTok Creative Center top ads URLs', () => {
    assert.doesNotThrow(() => requireTikTokAdUrl(url));
    assert.doesNotThrow(() => requireTikTokAdUrl(shortUrl));
    assert.doesNotThrow(() => requireTikTokAdUrl(`${url}?region=US&period=30`));
});

test('rejects non-Creative Center TikTok URLs', () => {
    assert.throws(
        () => requireTikTokAdUrl('https://www.tiktok.com/@tiktok/video/7568510388342443294'),
        /ads\.tiktok\.com/,
    );
    assert.throws(
        () => requireTikTokAdUrl('https://ads.tiktok.com/business/creativecenter/pc/en/topads/7221117041168252930/'),
        /topads\/\{ad_id\}\/pc\/en/,
    );
});

test('rejects non-TikTok and non-HTTPS URLs', () => {
    assert.throws(
        () => requireTikTokAdUrl('https://example.com/business/creativecenter/topads/7221117041168252930/'),
        /ads\.tiktok\.com/,
    );
    assert.throws(
        () => requireTikTokAdUrl('http://ads.tiktok.com/business/creativecenter/topads/7221117041168252930/'),
        /must use HTTPS/,
    );
});

test('requires at least one lookup', () => {
    assert.throws(
        () => resolveTikTokAdUrls({ urls: [], url: '' }),
        /At least one TikTok Creative Center ad URL is required/,
    );
});

test('input schema supports batch urls and legacy url', () => {
    const schema = JSON.parse(readFileSync(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
    const itemPattern = new RegExp(schema.properties.urls.items.pattern);
    const legacyPattern = new RegExp(schema.properties.url.pattern);

    assert.equal(itemPattern.test(url), true);
    assert.equal(itemPattern.test(shortUrl), true);
    assert.equal(itemPattern.test(`${url}?region=US`), true);
    assert.equal(legacyPattern.test(url), true);
    assert.equal(itemPattern.test('https://www.tiktok.com/@tiktok/video/7568510388342443294'), false);
    assert.equal(itemPattern.test('https://ads.tiktok.com/business/creativecenter/pc/en/topads/7221117041168252930/'), false);
});
