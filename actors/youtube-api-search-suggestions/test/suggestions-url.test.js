import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildSuggestionsRequest, suggestionsToDatasetItems } from '../src/suggestions-url.js';

describe('buildSuggestionsRequest', () => {
    it('builds the Scrappa YouTube suggestions URL', () => {
        const request = buildSuggestionsRequest({
            q: ' javascript tutorial ',
            hl: 'en',
            gl: 'us',
        });
        const url = new URL(request.url);

        assert.equal(request.query, 'javascript tutorial');
        assert.equal(request.hl, 'en');
        assert.equal(request.gl, 'US');
        assert.equal(url.origin + url.pathname, 'https://ytapi.scrappa.co/search/suggestions');
        assert.equal(url.searchParams.get('q'), 'javascript tutorial');
        assert.equal(url.searchParams.get('hl'), 'en');
        assert.equal(url.searchParams.get('gl'), 'US');
    });

    it('omits optional locale parameters when not provided', () => {
        const url = new URL(buildSuggestionsRequest({ q: 'news' }).url);

        assert.equal(url.searchParams.get('q'), 'news');
        assert.equal(url.searchParams.get('hl'), null);
        assert.equal(url.searchParams.get('gl'), null);
    });

    it('requires q', () => {
        assert.throws(() => buildSuggestionsRequest({ hl: 'en' }), /q/);
    });
});

describe('suggestionsToDatasetItems', () => {
    it('maps each suggestion to one dataset item', () => {
        const items = suggestionsToDatasetItems({
            query: 'javascript',
            locale: {
                hl: 'en',
                gl: 'US',
            },
            suggestions: ['javascript', 'javascript tutorial', ''],
        });

        assert.deepEqual(items, [
            {
                query: 'javascript',
                suggestion: 'javascript',
                position: 1,
                hl: 'en',
                gl: 'US',
            },
            {
                query: 'javascript',
                suggestion: 'javascript tutorial',
                position: 2,
                hl: 'en',
                gl: 'US',
            },
        ]);
    });

    it('uses request values when the API response omits metadata', () => {
        const items = suggestionsToDatasetItems({
            suggestions: ['one'],
        }, {
            query: 'fallback',
            hl: 'en',
            gl: 'US',
        });

        assert.deepEqual(items, [
            {
                query: 'fallback',
                suggestion: 'one',
                position: 1,
                hl: 'en',
                gl: 'US',
            },
        ]);
    });
});
