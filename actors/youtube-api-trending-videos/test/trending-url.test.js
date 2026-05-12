import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildTrendingRequest, continuationToken, trendingVideosToDatasetItems } from '../src/trending-url.js';

describe('buildTrendingRequest', () => {
    it('builds the Scrappa YouTube trending URL from Apify select arrays', () => {
        const request = buildTrendingRequest({
            category: [' music '],
            type: ['now'],
        });
        const url = new URL(request.url);

        assert.equal(request.category, 'music');
        assert.equal(request.type, 'now');
        assert.equal(url.origin + url.pathname, 'https://ytapi.scrappa.co/trending');
        assert.equal(url.searchParams.get('category'), 'music');
        assert.equal(url.searchParams.get('type'), 'now');
    });

    it('supports scalar string input values', () => {
        const url = new URL(buildTrendingRequest({ category: 'news', type: 'now' }).url);

        assert.equal(url.searchParams.get('category'), 'news');
        assert.equal(url.searchParams.get('type'), 'now');
    });

    it('omits empty optional parameters', () => {
        const request = buildTrendingRequest({
            category: [''],
            type: [],
        });

        assert.equal(request.url, 'https://ytapi.scrappa.co/trending');
    });

    it('uses the first non-null array value and ignores extras', () => {
        const url = new URL(buildTrendingRequest({
            category: [null, undefined, 'gaming', 'music'],
            type: [null, 'now'],
        }).url);

        assert.equal(url.searchParams.get('category'), 'gaming');
        assert.equal(url.searchParams.get('type'), 'now');
    });

    it('ignores non-string scalar values', () => {
        const request = buildTrendingRequest({
            category: 123,
            type: false,
        });

        assert.equal(request.url, 'https://ytapi.scrappa.co/trending');
    });
});

describe('trendingVideosToDatasetItems', () => {
    it('uses the current Scrappa results field', () => {
        const items = trendingVideosToDatasetItems({
            results: [{ id: 'one' }, { id: 'two' }],
        });

        assert.deepEqual(items, [{ id: 'one' }, { id: 'two' }]);
    });

    it('falls back to the legacy videos field', () => {
        const items = trendingVideosToDatasetItems({
            videos: [{ id: 'legacy' }],
        });

        assert.deepEqual(items, [{ id: 'legacy' }]);
    });

    it('returns an empty array when no result list is present', () => {
        assert.deepEqual(trendingVideosToDatasetItems({}), []);
    });

    it('returns an empty array when the result fields are not arrays', () => {
        assert.deepEqual(trendingVideosToDatasetItems({ results: {}, videos: {} }), []);
    });
});

describe('continuationToken', () => {
    it('reads the current pagination continuation token', () => {
        assert.equal(continuationToken({
            pagination: {
                continuationToken: 'next-page',
            },
        }), 'next-page');
    });

    it('falls back to the legacy continuation field', () => {
        assert.equal(continuationToken({ continuation: 'legacy-next-page' }), 'legacy-next-page');
    });

    it('prefers current pagination tokens over empty legacy continuation values', () => {
        assert.equal(continuationToken({
            continuation: '',
            pagination: {
                continuationToken: 'next-page',
            },
        }), 'next-page');
    });
});
