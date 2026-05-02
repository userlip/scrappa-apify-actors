import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildBatchVideosUrl } from '../src/videos-url.js';

describe('buildBatchVideosUrl', () => {
    it('builds a batch videos URL with comma-separated IDs', () => {
        const url = new URL(buildBatchVideosUrl({ ids: '7eul_Vt6SZY,6QQQKJJBJOY' }));

        assert.equal(url.origin + url.pathname, 'https://ytapi.scrappa.co/videos/batch');
        assert.equal(url.searchParams.get('ids'), '7eul_Vt6SZY,6QQQKJJBJOY');
    });

    it('requires ids', () => {
        assert.throws(() => buildBatchVideosUrl({}), /ids/);
        assert.throws(() => buildBatchVideosUrl({ ids: '   ' }), /ids/);
    });
});
