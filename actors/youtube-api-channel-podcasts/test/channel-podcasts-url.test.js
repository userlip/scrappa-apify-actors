import test from 'node:test';
import assert from 'node:assert/strict';

import { buildChannelPodcastsUrl } from '../src/channel-podcasts-url.js';

test('builds a channel podcasts URL with required id', () => {
    assert.equal(
        buildChannelPodcastsUrl({ id: 'UC example' }),
        'https://ytapi.scrappa.co/channels/podcasts?id=UC%20example',
    );
});

test('adds string sort and continuation parameters', () => {
    assert.equal(
        buildChannelPodcastsUrl({ id: 'UC123', sort: 'popular', continuation: 'next page' }),
        'https://ytapi.scrappa.co/channels/podcasts?id=UC123&sort=popular&continuation=next%20page',
    );
});

test('supports legacy single-item sort arrays', () => {
    assert.equal(
        buildChannelPodcastsUrl({ id: 'UC123', sort: ['newest'] }),
        'https://ytapi.scrappa.co/channels/podcasts?id=UC123&sort=newest',
    );
});

test('requires an id', () => {
    assert.throws(
        () => buildChannelPodcastsUrl({ sort: 'newest' }),
        /Search query "id" not provided/,
    );
});
