import test from 'node:test';
import assert from 'node:assert/strict';

import { buildRelatedVideosUrl } from '../src/related-videos-url.js';

test('builds a related videos URL with required id', () => {
    assert.equal(
        buildRelatedVideosUrl({ id: 'dQw4w9WgXcQ' }),
        'https://ytapi.scrappa.co/videos/related?id=dQw4w9WgXcQ',
    );
});

test('encodes the video id', () => {
    assert.equal(
        buildRelatedVideosUrl({ id: 'video id' }),
        'https://ytapi.scrappa.co/videos/related?id=video%20id',
    );
});

test('adds continuation token when provided', () => {
    assert.equal(
        buildRelatedVideosUrl({ id: 'dQw4w9WgXcQ', continuation: 'next page/token' }),
        'https://ytapi.scrappa.co/videos/related?id=dQw4w9WgXcQ&continuation=next%20page%2Ftoken',
    );
});

test('ignores blank continuation tokens', () => {
    assert.equal(
        buildRelatedVideosUrl({ id: 'dQw4w9WgXcQ', continuation: '   ' }),
        'https://ytapi.scrappa.co/videos/related?id=dQw4w9WgXcQ',
    );
});

test('requires an id', () => {
    assert.throws(
        () => buildRelatedVideosUrl({}),
        /YouTube video ID "id" not provided/,
    );
});
