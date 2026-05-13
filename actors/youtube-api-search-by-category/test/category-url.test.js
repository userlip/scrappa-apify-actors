import assert from 'node:assert/strict';
import fs from 'node:fs';
import { describe, it } from 'node:test';
import { buildCategoryRequest, categoryVideosToDatasetItems, continuationToken } from '../src/category-url.js';
import { errorMessage } from '../src/errors.js';

const inputSchema = JSON.parse(fs.readFileSync(new URL('../.actor/input_schema.json', import.meta.url)));

describe('buildCategoryRequest', () => {
    it('builds the Scrappa YouTube category URL from Apify select arrays', () => {
        const request = buildCategoryRequest({
            category: [' education '],
            sort: ['view_count'],
            duration: ['short'],
            upload_date: ['week'],
            limit: 5,
            continuation: 'next-page',
            contentType: ['live'],
            features: 'hd, cc',
        });
        const url = new URL(request.url);

        assert.equal(request.category, 'education');
        assert.equal(url.origin + url.pathname, 'https://ytapi.scrappa.co/search/category');
        assert.equal(url.searchParams.get('category'), 'education');
        assert.equal(url.searchParams.get('sort'), 'view_count');
        assert.equal(url.searchParams.get('duration'), 'short');
        assert.equal(url.searchParams.get('upload_date'), 'week');
        assert.equal(url.searchParams.get('limit'), '5');
        assert.equal(url.searchParams.get('continuation'), 'next-page');
        assert.equal(url.searchParams.get('contentType'), 'live');
        assert.equal(url.searchParams.get('features'), 'hd,cc');
    });

    it('supports scalar string input values', () => {
        const url = new URL(buildCategoryRequest({ category: 'music', sort: 'rating' }).url);

        assert.equal(url.searchParams.get('category'), 'music');
        assert.equal(url.searchParams.get('sort'), 'rating');
    });

    it('requires a category', () => {
        assert.throws(
            () => buildCategoryRequest({ category: [''] }),
            /Search category is required/,
        );
    });

    it('uses the only non-empty string from array values', () => {
        const url = new URL(buildCategoryRequest({
            category: [null, undefined, '', 'gaming'],
            sort: [null, 'relevance'],
        }).url);

        assert.equal(url.searchParams.get('category'), 'gaming');
        assert.equal(url.searchParams.get('sort'), 'relevance');
    });

    it('rejects multiple selected values for single-choice filters', () => {
        assert.throws(
            () => buildCategoryRequest({ category: ['gaming', 'music'] }),
            /category accepts only one selected value/,
        );
        assert.throws(
            () => buildCategoryRequest({ category: ['gaming'], sort: ['rating', 'relevance'] }),
            /sort accepts only one selected value/,
        );
    });

    it('rejects limits above the endpoint maximum', () => {
        assert.throws(
            () => buildCategoryRequest({ category: 'education', limit: 1024 }),
            /Limit must be less than or equal to 20/,
        );
    });

    it('omits empty optional parameters and defaults sort', () => {
        const url = new URL(buildCategoryRequest({
            category: 'education',
            duration: [],
            upload_date: '',
            continuation: '  ',
            features: '',
        }).url);

        assert.equal(url.searchParams.get('sort'), 'relevance');
        assert.equal(url.searchParams.has('duration'), false);
        assert.equal(url.searchParams.has('upload_date'), false);
        assert.equal(url.searchParams.has('continuation'), false);
        assert.equal(url.searchParams.has('features'), false);
    });

    it('normalizes comma-separated feature values from strings and arrays', () => {
        const stringUrl = new URL(buildCategoryRequest({
            category: 'education',
            features: ' hd, ,cc, 4k ',
        }).url);
        const arrayUrl = new URL(buildCategoryRequest({
            category: 'education',
            features: [' hd,cc ', '', ' 4k '],
        }).url);

        assert.equal(stringUrl.searchParams.get('features'), 'hd,cc,4k');
        assert.equal(arrayUrl.searchParams.get('features'), 'hd,cc,4k');
    });
});

describe('categoryVideosToDatasetItems', () => {
    it('uses the current Scrappa results field', () => {
        assert.deepEqual(categoryVideosToDatasetItems({
            results: [{ id: 'one' }, { id: 'two' }],
        }), [{ id: 'one' }, { id: 'two' }]);
    });

    it('falls back to the legacy videos field', () => {
        assert.deepEqual(categoryVideosToDatasetItems({
            videos: [{ id: 'legacy' }],
        }), [{ id: 'legacy' }]);
    });

    it('returns an empty array when no result list is present', () => {
        assert.deepEqual(categoryVideosToDatasetItems({}), []);
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
});

describe('input schema', () => {
    it('keeps Apify select fields aligned with runtime single-value handling', () => {
        for (const field of ['category', 'sort', 'duration', 'upload_date', 'contentType']) {
            assert.equal(inputSchema.properties[field].maxItems, 1);
        }

        assert.equal(inputSchema.properties.category.minItems, 1);
    });
});

describe('errorMessage', () => {
    it('returns a timeout message for AbortSignal timeout errors', () => {
        const error = new DOMException('The operation was aborted', 'TimeoutError');

        assert.match(errorMessage(error), /timed out after 60s/);
    });

    it('returns the raw message for non-timeout errors', () => {
        assert.equal(errorMessage(new Error('something broke')), 'something broke');
    });
});
