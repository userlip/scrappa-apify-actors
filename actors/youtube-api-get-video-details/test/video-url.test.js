import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildVideoDetailsUrl } from '../src/video-url.js';

describe('buildVideoDetailsUrl', () => {
    it('builds a video details URL with the video ID', () => {
        const url = new URL(buildVideoDetailsUrl({ id: 'dQw4w9WgXcQ' }));

        assert.equal(url.origin + url.pathname, 'https://ytapi.scrappa.co/videos');
        assert.equal(url.searchParams.get('id'), 'dQw4w9WgXcQ');
    });

    it('requires id', () => {
        assert.throws(() => buildVideoDetailsUrl({}), /id/);
        assert.throws(() => buildVideoDetailsUrl({ id: '   ' }), /id/);
    });
});
