import test from 'node:test';
import assert from 'node:assert/strict';

import { assertContinuationMatchesBatch, buildChannelPodcastsUrl, getChannelIds } from '../src/channel-podcasts-url.js';
import { buildScrappaRequest, getScrappaApiKey } from '../src/youtube-request.js';

test('builds a channel podcasts URL with required id', () => {
    assert.equal(
        buildChannelPodcastsUrl({ id: 'UC example' }),
        'https://scrappa.co/api/youtube/channel-videos?channel_id=UC+example',
    );
});

test('adds string sort and continuation parameters', () => {
    assert.equal(
        buildChannelPodcastsUrl({ id: 'UC123', sort: 'popular', continuation: 'next page' }),
        'https://scrappa.co/api/youtube/channel-videos?channel_id=UC123&sort=popular&continuation=next+page',
    );
});

test('supports legacy single-item sort arrays', () => {
    assert.equal(
        buildChannelPodcastsUrl({ id: 'UC123', sort: ['newest'] }),
        'https://scrappa.co/api/youtube/channel-videos?channel_id=UC123&sort=newest',
    );
});

test('parses batch channel IDs from ids and legacy id', () => {
    assert.deepEqual(
        getChannelIds({ ids: 'UC1, UC2', id: 'UC2,UC3' }),
        ['UC1', 'UC2', 'UC3'],
    );
});

test('rejects continuation tokens with batch channel IDs', () => {
    assert.throws(
        () => assertContinuationMatchesBatch({ ids: 'UC1,UC2', continuation: 'next page' }),
        /continuation/,
    );
});

test('builds authenticated Scrappa request options', () => {
    const { requestOptions } = buildScrappaRequest('https://scrappa.co/api/youtube/channel-videos?channel_id=UC1', 'secret-key', 1000);

    assert.equal(requestOptions.headers['X-API-Key'], 'secret-key');
    assert.equal(requestOptions.headers.Accept, 'application/json');
    assert.ok(requestOptions.signal instanceof AbortSignal);
});

test('requires SCRAPPA_API_KEY', () => {
    assert.equal(getScrappaApiKey({ SCRAPPA_API_KEY: 'test-key' }), 'test-key');
    assert.throws(() => getScrappaApiKey({}), /SCRAPPA_API_KEY environment variable is not set/);
});

test('requires an id', () => {
    assert.throws(
        () => buildChannelPodcastsUrl({ sort: 'newest' }),
        /Search query "id" not provided/,
    );
});
