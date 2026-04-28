import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildSearchUrl } from '../src/search-url.js';

describe('buildSearchUrl', () => {
    it('binds Apify select arrays to the correct Scrappa search params', () => {
        const url = new URL(buildSearchUrl({
            q: 'javascript tutorial',
            sort: ['view_count'],
            duration: ['medium'],
            upload_date: ['week'],
            limit: 25,
            continuation: 'next-page',
            contentType: ['recorded'],
            features: 'hd,subtitles',
            type: ['video'],
        }));

        assert.equal(url.origin + url.pathname, 'https://ytapi.scrappa.co/search');
        assert.equal(url.searchParams.get('q'), 'javascript tutorial');
        assert.equal(url.searchParams.get('sort'), 'view_count');
        assert.equal(url.searchParams.get('duration'), 'medium');
        assert.equal(url.searchParams.get('upload_date'), 'week');
        assert.equal(url.searchParams.get('limit'), '25');
        assert.equal(url.searchParams.get('continuation'), 'next-page');
        assert.equal(url.searchParams.get('contentType'), 'recorded');
        assert.equal(url.searchParams.get('features'), 'hd,subtitles');
        assert.equal(url.searchParams.get('type'), 'video');
    });

    it('does not map filter values into limit when using the actor input shape', () => {
        const url = new URL(buildSearchUrl({
            q: 'news',
            sort: ['relevance'],
            duration: ['short'],
            upload_date: ['today'],
            limit: 2,
        }));

        assert.equal(url.searchParams.get('duration'), 'short');
        assert.equal(url.searchParams.get('upload_date'), 'today');
        assert.equal(url.searchParams.get('limit'), '2');
    });

    it('requires q', () => {
        assert.throws(() => buildSearchUrl({ sort: ['relevance'] }), /q/);
    });

    it('handles missing actor input with a validation error', () => {
        assert.throws(() => buildSearchUrl(null), /q/);
    });
});
