import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildSearchRequest } from '../src/search-url.js';

describe('buildSearchRequest', () => {
    it('binds Apify select arrays to the correct Scrappa search params', () => {
        const request = buildSearchRequest({
            q: 'javascript tutorial',
            sort: ['view_count'],
            duration: ['medium'],
            upload_date: ['week'],
            limit: 25,
            continuation: 'next-page',
            type: ['video'],
        }, {
            now: new Date('2026-05-02T12:00:00.000Z'),
        });
        const url = new URL(request.url);

        assert.equal(request.query, 'javascript tutorial');
        assert.equal(url.origin + url.pathname, 'https://scrappa.co/api/youtube/search');
        assert.equal(url.searchParams.get('query'), 'javascript tutorial');
        assert.equal(url.searchParams.get('order'), 'viewCount');
        assert.equal(url.searchParams.get('videoDuration'), 'medium');
        assert.equal(url.searchParams.get('publishedAfter'), '2026-04-25T12:00:00.000Z');
        assert.equal(url.searchParams.get('upload_date'), null);
        assert.equal(url.searchParams.get('limit'), '20');
        assert.equal(url.searchParams.get('continuation'), 'next-page');
        assert.equal(url.searchParams.get('type'), 'video');
    });

    it('does not forward unsupported Scrappa search filters', () => {
        const url = new URL(buildSearchRequest({
            q: 'live 4k videos',
            contentType: ['live'],
            features: '4k,hd',
        }).url);

        assert.equal(url.searchParams.get('contentType'), null);
        assert.equal(url.searchParams.get('features'), null);
    });

    it('does not map filter values into limit when using the actor input shape', () => {
        const url = new URL(buildSearchRequest({
            q: 'news',
            sort: ['relevance'],
            duration: ['short'],
            upload_date: ['today'],
            limit: 2,
        }, {
            now: new Date('2026-05-02T12:00:00.000Z'),
        }).url);

        assert.equal(url.searchParams.get('order'), 'relevance');
        assert.equal(url.searchParams.get('videoDuration'), 'short');
        assert.equal(url.searchParams.get('publishedAfter'), '2026-05-01T12:00:00.000Z');
        assert.equal(url.searchParams.get('limit'), '2');
    });

    it('caps limit to the Scrappa YouTube search maximum', () => {
        const url = new URL(buildSearchRequest({
            q: 'news',
            limit: 1024,
        }).url);

        assert.equal(url.searchParams.get('limit'), '20');
    });

    it('maps upload date buckets to publishedAfter filters', () => {
        const now = new Date('2026-05-02T12:00:00.000Z');

        assert.equal(new URL(buildSearchRequest({ q: 'hour', upload_date: ['hour'] }, { now }).url).searchParams.get('publishedAfter'), '2026-05-02T11:00:00.000Z');
        assert.equal(new URL(buildSearchRequest({ q: 'today', upload_date: ['today'] }, { now }).url).searchParams.get('publishedAfter'), '2026-05-01T12:00:00.000Z');
        assert.equal(new URL(buildSearchRequest({ q: 'week', upload_date: ['week'] }, { now }).url).searchParams.get('publishedAfter'), '2026-04-25T12:00:00.000Z');
        assert.equal(new URL(buildSearchRequest({ q: 'month', upload_date: ['month'] }, { now }).url).searchParams.get('publishedAfter'), '2026-04-02T12:00:00.000Z');
        assert.equal(new URL(buildSearchRequest({ q: 'year', upload_date: ['year'] }, { now }).url).searchParams.get('publishedAfter'), '2025-05-02T12:00:00.000Z');
    });

    it('uses calendar month and year boundaries for upload date filters', () => {
        const monthEnd = new Date('2026-03-31T12:00:00.000Z');
        const leapDay = new Date('2024-02-29T12:00:00.000Z');

        assert.equal(new URL(buildSearchRequest({ q: 'month', upload_date: ['month'] }, { now: monthEnd }).url).searchParams.get('publishedAfter'), '2026-02-28T12:00:00.000Z');
        assert.equal(new URL(buildSearchRequest({ q: 'year', upload_date: ['year'] }, { now: leapDay }).url).searchParams.get('publishedAfter'), '2023-02-28T12:00:00.000Z');
    });

    it('maps upload date sort to the Scrappa order parameter', () => {
        const url = new URL(buildSearchRequest({
            q: 'new videos',
            sort: ['upload_date'],
        }).url);

        assert.equal(url.searchParams.get('order'), 'date');
    });

    it('passes through Scrappa-compatible sort values without translation', () => {
        const url = new URL(buildSearchRequest({
            q: 'top rated videos',
            sort: ['rating'],
        }).url);

        assert.equal(url.searchParams.get('order'), 'rating');
    });

    it('requires q', () => {
        assert.throws(() => buildSearchRequest({ sort: ['relevance'] }), /q/);
    });

    it('handles missing actor input with a validation error', () => {
        assert.throws(() => buildSearchRequest(null), /q/);
    });
});
