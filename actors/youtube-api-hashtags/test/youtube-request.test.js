import test from 'node:test';
import assert from 'node:assert/strict';

import {
    buildHashtagSearchUrl,
    buildScrappaRequest,
    getScrappaApiKey,
} from '../src/youtube-request.js';

test('builds maintained hashtag search URL', () => {
    assert.equal(
        buildHashtagSearchUrl({ hashtag: 'javascript', sort: 'upload_date', limit: 50, duration: 'short', continuation: 'next page' }),
        'https://scrappa.co/api/youtube/search?query=%23javascript&order=date&limit=20&videoDuration=short&continuation=next+page',
    );
});

test('caps hashtag search limit to Scrappa maximum', () => {
    const url = new URL(buildHashtagSearchUrl({ hashtag: '#javascript', limit: 50 }));

    assert.equal(url.searchParams.get('query'), '#javascript');
    assert.equal(url.searchParams.get('limit'), '20');
});

test('builds authenticated request options', () => {
    const { requestOptions } = buildScrappaRequest('https://scrappa.co/api/youtube/search?query=%23javascript', 'secret-key');

    assert.equal(requestOptions.headers['X-API-Key'], 'secret-key');
    assert.equal(requestOptions.headers.Accept, 'application/json');
});

test('requires SCRAPPA_API_KEY', () => {
    assert.equal(getScrappaApiKey({ SCRAPPA_API_KEY: 'test-key' }), 'test-key');
    assert.throws(() => getScrappaApiKey({}), /SCRAPPA_API_KEY environment variable is not set/);
});
