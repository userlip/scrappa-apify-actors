import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import { fetchBatchVideos } from '../src/fetch-videos.js';
import { buildBatchVideosUrl } from '../src/videos-url.js';

const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
const prefilledUrl = buildBatchVideosUrl({ ids: schema.properties.ids.prefill });

function response(status, videos) {
    return {
        ok: status === 200,
        status,
        statusText: status === 504 ? 'Gateway Timeout' : 'Bad Request',
        json: async () => ({ videos }),
    };
}

describe('prefilled batch video request', () => {
    it('retries the QA run’s 504 and returns videos for its schema prefill', async () => {
        const requests = [];
        const waits = [];
        const videos = [{ id: '7eul_Vt6SZY' }, { id: '6QQQKJJBJOY' }];
        const fetcher = async (url, options) => {
            requests.push({ url, options });
            return requests.length === 1 ? response(504) : response(200, videos);
        };

        assert.deepEqual(await fetchBatchVideos(prefilledUrl, fetcher, async (ms) => waits.push(ms)), videos);
        assert.equal(requests.length, 2);
        assert.equal(requests[0].url, requests[1].url);
        assert.equal(new URL(requests[0].url).searchParams.get('ids'), schema.properties.ids.prefill);
        assert.ok(requests.every(({ options }) => options.signal instanceof AbortSignal));
        assert.deepEqual(waits, [1000]);
    });

    it('fails after three persistent 504 responses', async () => {
        let calls = 0;
        await assert.rejects(fetchBatchVideos(prefilledUrl, async () => {
            calls += 1;
            return response(504);
        }, async () => {}), /504 Gateway Timeout/);
        assert.equal(calls, 3);
    });

    it('retries transport failures, but stops after three attempts', async () => {
        let calls = 0;
        await assert.rejects(fetchBatchVideos(prefilledUrl, async () => {
            calls += 1;
            throw new Error('upstream connection reset');
        }, async () => {}), /upstream connection reset/);
        assert.equal(calls, 3);
    });

    it('does not retry bad input or malformed successful responses', async () => {
        let calls = 0;
        const fetcher = async () => { calls += 1; return response(400); };
        await assert.rejects(fetchBatchVideos(prefilledUrl, fetcher, async () => {}), /400 Bad Request/);
        assert.equal(calls, 1);
        await assert.rejects(fetchBatchVideos(prefilledUrl, async () => response(200), async () => {}), /missing the videos array/);
    });
});
