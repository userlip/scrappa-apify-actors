import test from 'node:test';
import assert from 'node:assert/strict';

import {
    assertContinuationMatchesBatch,
    buildChannelShortsUrl,
    buildScrappaRequest,
    collectFilteredChannelVideos,
    getChannelIds,
    getScrappaApiKey,
} from '../src/youtube-request.js';

test('builds maintained channel shorts URL', () => {
    assert.equal(
        buildChannelShortsUrl({ id: 'UC example', sort: 'popular', continuation: 'next page' }),
        'https://scrappa.co/api/youtube/channel-videos?channel_id=UC+example&sort=popular&continuation=next+page',
    );
});

test('parses batch channel IDs', () => {
    assert.deepEqual(getChannelIds({ ids: 'UC1, UC2', id: 'UC2,UC3' }), ['UC1', 'UC2', 'UC3']);
});

test('rejects continuation tokens with batch channel IDs', () => {
    assert.throws(
        () => assertContinuationMatchesBatch({ ids: 'UC1,UC2', continuation: 'next page' }),
        /continuation/,
    );
});

test('collects Shorts across mixed channel upload pages', async () => {
    const requestedContinuations = [];
    const pages = [
        {
            videos: [{ id: 'regular-1', type: 'video' }],
            pagination: { continuationToken: 'page-2' },
        },
        {
            videos: [{ id: 'short-1', videoType: 'short' }],
            pagination: { continuationToken: 'page-3' },
        },
    ];

    const result = await collectFilteredChannelVideos(
        { id: 'UC1' },
        async (input) => {
            requestedContinuations.push(input.continuation ?? '');
            return pages.shift();
        },
        (video) => String(video?.type ?? video?.videoType ?? '').toLowerCase() === 'short',
        { targetResultCount: 1 },
    );

    assert.deepEqual(requestedContinuations, ['', 'page-2']);
    assert.deepEqual(result.videos, [{ id: 'short-1', videoType: 'short' }]);
    assert.equal(result.continuation, 'page-3');
});

test('builds authenticated request options', () => {
    const { requestOptions } = buildScrappaRequest('https://scrappa.co/api/youtube/channel-videos?channel_id=UC1', 'secret-key');

    assert.equal(requestOptions.headers['X-API-Key'], 'secret-key');
    assert.equal(requestOptions.headers.Accept, 'application/json');
});

test('requires SCRAPPA_API_KEY', () => {
    assert.equal(getScrappaApiKey({ SCRAPPA_API_KEY: 'test-key' }), 'test-key');
    assert.throws(() => getScrappaApiKey({}), /SCRAPPA_API_KEY environment variable is not set/);
});
