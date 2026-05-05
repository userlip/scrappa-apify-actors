import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { buildTranscriptRequest, fetchTranscript } from '../src/transcript-client.js';

describe('buildTranscriptRequest', () => {
    it('adds Scrappa API headers', () => {
        const { requestOptions } = buildTranscriptRequest({ id: 'dQw4w9WgXcQ' }, 'secret-key');

        assert.equal(requestOptions.headers['X-API-Key'], 'secret-key');
        assert.equal(requestOptions.headers.Accept, 'application/json');
    });
});

describe('fetchTranscript', () => {
    it('returns parsed JSON from Scrappa', async () => {
        const result = await fetchTranscript({ id: 'dQw4w9WgXcQ' }, {
            apiKey: 'secret-key',
            fetchFn: async () => ({
                ok: true,
                json: async () => ({ videoId: 'dQw4w9WgXcQ', transcript: [] }),
            }),
        });

        assert.equal(result.data.videoId, 'dQw4w9WgXcQ');
    });

    it('throws on non-2xx responses', async () => {
        await assert.rejects(
            () => fetchTranscript({ id: 'dQw4w9WgXcQ' }, {
                apiKey: 'secret-key',
                fetchFn: async () => ({
                    ok: false,
                    status: 401,
                    statusText: 'Unauthorized',
                }),
            }),
            /401 Unauthorized/,
        );
    });
});
