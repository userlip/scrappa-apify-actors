import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildVideoCommentsRequest, fetchVideoComments } from '../src/comments-client.js';

describe('buildVideoCommentsRequest', () => {
    it('builds Scrappa comments API request with API key auth headers', () => {
        const { apiUrl, requestOptions } = buildVideoCommentsRequest(
            { id: 'dQw4w9WgXcQ', sort: ['TOP_COMMENTS'] },
            'test-key'
        );

        const url = new URL(apiUrl);
        assert.equal(url.origin + url.pathname, 'https://scrappa.co/api/youtube/comments');
        assert.equal(url.searchParams.get('video_id'), 'dQw4w9WgXcQ');
        assert.equal(requestOptions.headers['X-API-Key'], 'test-key');
        assert.equal(requestOptions.headers.Accept, 'application/json');
    });
});

describe('fetchVideoComments', () => {
    it('returns parsed JSON from Scrappa comments API', async () => {
        let loggedUrl;
        const fetchFn = async () => {
            return {
                ok: true,
                async json() {
                    return { comments: [{ id: 'comment-1' }] };
                },
            };
        };

        const result = await fetchVideoComments(
            { id: 'dQw4w9WgXcQ' },
            {
                apiKey: 'test-key',
                fetchFn,
                onRequest: (apiUrl) => {
                    loggedUrl = apiUrl;
                },
            }
        );

        assert.equal(new URL(loggedUrl).searchParams.get('video_id'), 'dQw4w9WgXcQ');
        assert.equal(result.data.comments[0].id, 'comment-1');
    });

    it('throws an actionable error for non-2xx Scrappa responses', async () => {
        const fetchFn = async () => ({
            ok: false,
            status: 401,
            statusText: 'Unauthorized',
        });

        await assert.rejects(
            () => fetchVideoComments({ id: 'dQw4w9WgXcQ' }, { apiKey: 'bad-key', fetchFn }),
            /Scrappa API request failed with 401 Unauthorized/
        );
    });
});
