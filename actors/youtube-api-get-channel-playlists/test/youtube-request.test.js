import test from 'node:test';
import assert from 'node:assert/strict';

import {
    buildChannelPlaylistsUrl,
    buildScrappaRequest,
    getChannelIds,
    getScrappaApiKey,
} from '../src/youtube-request.js';

test('builds maintained channel playlists URL', () => {
    assert.equal(
        buildChannelPlaylistsUrl({ id: 'UC example', continuation: 'next page' }),
        'https://scrappa.co/api/youtube/channel-playlists?channel_id=UC+example&continuation=next+page',
    );
});

test('parses batch channel IDs', () => {
    assert.deepEqual(getChannelIds({ ids: 'UC1, UC2', id: 'UC2,UC3' }), ['UC1', 'UC2', 'UC3']);
});

test('builds authenticated request options', () => {
    const { requestOptions } = buildScrappaRequest('https://scrappa.co/api/youtube/channel-playlists?channel_id=UC1', 'secret-key');

    assert.equal(requestOptions.headers['X-API-Key'], 'secret-key');
    assert.equal(requestOptions.headers.Accept, 'application/json');
});

test('requires SCRAPPA_API_KEY', () => {
    assert.equal(getScrappaApiKey({ SCRAPPA_API_KEY: 'test-key' }), 'test-key');
    assert.throws(() => getScrappaApiKey({}), /SCRAPPA_API_KEY environment variable is not set/);
});
