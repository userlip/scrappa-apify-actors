import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildVideoCommentsUrl } from '../src/comments-url.js';

describe('buildVideoCommentsUrl', () => {
    it('builds a comments URL with Apify select-array sort input', () => {
        const url = new URL(buildVideoCommentsUrl({
            id: 'dQw4w9WgXcQ',
            sort: ['NEWEST_FIRST'],
            continuation: 'next-page',
        }));

        assert.equal(url.origin + url.pathname, 'https://ytapi.scrappa.co/videos/comments');
        assert.equal(url.searchParams.get('id'), 'dQw4w9WgXcQ');
        assert.equal(url.searchParams.get('sort'), 'NEWEST_FIRST');
        assert.equal(url.searchParams.get('continuation'), 'next-page');
    });

    it('accepts string sort input', () => {
        const url = new URL(buildVideoCommentsUrl({ id: 'dQw4w9WgXcQ', sort: 'TOP_COMMENTS' }));

        assert.equal(url.searchParams.get('sort'), 'TOP_COMMENTS');
    });

    it('requires id', () => {
        assert.throws(() => buildVideoCommentsUrl({}), /id/);
        assert.throws(() => buildVideoCommentsUrl({ id: '   ' }), /id/);
    });
});
