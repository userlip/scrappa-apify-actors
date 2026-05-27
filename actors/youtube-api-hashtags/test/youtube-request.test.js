import test from 'node:test';
import assert from 'node:assert/strict';

import {
    buildHashtagSearchUrl,
    buildScrappaRequest,
    getContinuationToken,
    getScrappaApiKey,
} from '../src/youtube-request.js';

test('builds maintained hashtag search URL', () => {
    assert.equal(
        buildHashtagSearchUrl({ hashtag: 'javascript', sort: 'upload_date', limit: 50, duration: 'short', continuation: 'next page' }),
        'https://scrappa.co/api/youtube/search?query=%23javascript&type=video&order=date&limit=20&videoDuration=short&continuation=next+page',
    );
});

test('caps hashtag search limit to Scrappa maximum', () => {
    const url = new URL(buildHashtagSearchUrl({ hashtag: '#javascript', limit: 50 }));

    assert.equal(url.searchParams.get('query'), '#javascript');
    assert.equal(url.searchParams.get('type'), 'video');
    assert.equal(url.searchParams.get('limit'), '20');
});

test('maps upload_date to publishedAfter for hashtag search', () => {
    const url = new URL(buildHashtagSearchUrl({
        hashtag: 'javascript',
        upload_date: 'week',
        contentType: 'live',
        features: 'hd,subtitles',
    }, { now: new Date('2026-05-27T12:00:00.000Z') }));

    assert.equal(url.searchParams.get('publishedAfter'), '2026-05-20T12:00:00.000Z');
    assert.equal(url.searchParams.has('upload_date'), false);
    assert.equal(url.searchParams.get('contentType'), 'live');
    assert.equal(url.searchParams.get('features'), 'hd,subtitles');
});

test('builds authenticated request options', () => {
    const { requestOptions } = buildScrappaRequest('https://scrappa.co/api/youtube/search?query=%23javascript', 'secret-key');

    assert.equal(requestOptions.headers['X-API-Key'], 'secret-key');
    assert.equal(requestOptions.headers.Accept, 'application/json');
});

test('reads hashtag search continuation tokens from Scrappa response shapes', () => {
    assert.equal(getContinuationToken({ pagination: { continuationToken: 'next-page' } }), 'next-page');
    assert.equal(getContinuationToken({ continuation: 'legacy-next-page' }), 'legacy-next-page');
});

test('requires SCRAPPA_API_KEY', () => {
    assert.equal(getScrappaApiKey({ SCRAPPA_API_KEY: 'test-key' }), 'test-key');
    assert.throws(() => getScrappaApiKey({}), /SCRAPPA_API_KEY environment variable is not set/);
});
