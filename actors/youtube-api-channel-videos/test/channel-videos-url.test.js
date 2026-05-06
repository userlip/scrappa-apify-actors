import test from 'node:test';
import assert from 'node:assert/strict';

import { buildChannelVideosUrl } from '../src/channel-videos-url.js';

test('builds a channel videos URL with required id', () => {
    assert.equal(
        buildChannelVideosUrl({ id: 'UC example' }),
        'https://ytapi.scrappa.co/channels/videos?id=UC%20example',
    );
});

test('adds string sort and continuation parameters', () => {
    assert.equal(
        buildChannelVideosUrl({ id: 'UC123', sort: 'popular', continuation: 'next page' }),
        'https://ytapi.scrappa.co/channels/videos?id=UC123&sort=popular&continuation=next%20page',
    );
});

test('supports legacy single-item sort arrays', () => {
    assert.equal(
        buildChannelVideosUrl({ id: 'UC123', sort: ['newest'] }),
        'https://ytapi.scrappa.co/channels/videos?id=UC123&sort=newest',
    );
});

test('requires an id', () => {
    assert.throws(
        () => buildChannelVideosUrl({ sort: 'newest' }),
        /Search query "id" not provided/,
    );
});
