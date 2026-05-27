import test from 'node:test';
import assert from 'node:assert/strict';

import {
    buildPlaylistSearchUrl,
    buildScrappaRequest,
    getContinuationToken,
    getScrappaApiKey,
} from '../src/youtube-request.js';

test('builds maintained playlist search URL', () => {
    assert.equal(
        buildPlaylistSearchUrl({ q: 'music mix', sort: 'view_count', limit: 50, continuation: 'next page' }),
        'https://scrappa.co/api/youtube/search?query=music+mix&type=playlist&order=viewCount&limit=20&continuation=next+page',
    );
});

test('caps playlist search limit to Scrappa maximum', () => {
    const url = new URL(buildPlaylistSearchUrl({ q: 'music mix', limit: 50 }));

    assert.equal(url.searchParams.get('limit'), '20');
});

test('builds authenticated request options', () => {
    const { requestOptions } = buildScrappaRequest('https://scrappa.co/api/youtube/search?query=test&type=playlist', 'secret-key');

    assert.equal(requestOptions.headers['X-API-Key'], 'secret-key');
    assert.equal(requestOptions.headers.Accept, 'application/json');
});

test('reads playlist search continuation tokens from Scrappa response shapes', () => {
    assert.equal(getContinuationToken({ pagination: { continuationToken: 'next-page' } }), 'next-page');
    assert.equal(getContinuationToken({ continuation: 'legacy-next-page' }), 'legacy-next-page');
});

test('requires SCRAPPA_API_KEY', () => {
    assert.equal(getScrappaApiKey({ SCRAPPA_API_KEY: 'test-key' }), 'test-key');
    assert.throws(() => getScrappaApiKey({}), /SCRAPPA_API_KEY environment variable is not set/);
});
