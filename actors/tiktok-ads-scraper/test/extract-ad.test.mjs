import assert from 'node:assert/strict';
import test from 'node:test';

import { extractSingleTikTokAdRecord } from '../dist/extract-ad.js';

const url = 'https://ads.tiktok.com/business/creativecenter/topads/7213160569871581185/pc/en';

test('extracts a single object response', () => {
    const ad = { id: '7213160569871581185' };

    assert.equal(extractSingleTikTokAdRecord(ad, url), ad);
});

test('extracts a single item array response', () => {
    const ad = { id: '7213160569871581185' };

    assert.equal(extractSingleTikTokAdRecord([ad], url), ad);
});

test('returns null and warns for empty array responses', () => {
    const warnings = [];

    assert.equal(extractSingleTikTokAdRecord([], url, (message) => warnings.push(message)), null);
    assert.equal(warnings.length, 1);
    assert.match(warnings[0], /empty ad record array/);
});

test('throws when Scrappa returns multiple ad records for one URL', () => {
    assert.throws(
        () => extractSingleTikTokAdRecord([{ id: 'one' }, { id: 'two' }], url),
        /Expected exactly one ad record/,
    );
});
